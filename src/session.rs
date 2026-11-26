//! The FamilySearch session, delivered once and kept only in memory.
//!
//! The token never goes to disk. A holder process keeps it and hands it to the
//! other commands over a Unix socket inside the state directory.

mod oauth;

use std::{
    env, fmt,
    fs::{self, Permissions},
    io::{self, BufRead, BufReader, ErrorKind, IsTerminal, Read, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Context, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use serde_json::json;

use crate::{
    cli::{Config, SessionCommand},
    error::Error,
    pacing::Turn,
    transport::Transport,
};

/// The session token: a single line with no spaces, ready to go in a header.
#[derive(PartialEq, Eq)]
pub(crate) struct Token(String);

impl Token {
    /// Returns `Error::Input` if the text, once trimmed, is empty or has spaces
    /// or control characters.
    pub(crate) fn new(raw: &str) -> Result<Self, Error> {
        let value = raw.trim();
        if value.is_empty() || !value.chars().all(|c| c.is_ascii_graphic()) {
            return Err(Error::Input(
                "o token da sessão deve ser uma linha só, sem espaços".into(),
            ));
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use = "the token is only useful to authenticate a request"]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

// The token must not show up in error messages or logs.
impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(***)")
    }
}

/// 32 random bytes in base64url: 43 characters, the minimum for a PKCE
/// verifier, and enough for a secret nobody guesses.
pub(crate) fn random_string() -> Result<String, Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).context("o sistema não forneceu bytes aleatórios")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// The tree contributor and person identifiers from the current user profile.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentUser {
    pub(crate) tree_user_id: Option<String>,
    person_id: Option<String>,
}

impl CurrentUser {
    pub(crate) fn fetch(transport: &Transport) -> Result<Self, Error> {
        #[derive(Deserialize)]
        struct Users {
            users: [CurrentUser; 1],
        }
        let reply = transport.get("/platform/users/current")?;
        let Users { users: [user] } = serde_json::from_str(&reply.body)
            .map_err(|_| anyhow!("a resposta do FamilySearch não identifica o usuário atual"))?;
        Ok(user)
    }
}

/// Runs a command of the `sessao` group.
pub(crate) fn run(config: &Config, command: &SessionCommand) -> Result<(), Error> {
    match command {
        SessionCommand::Deliver => {
            let mut stdin = io::stdin();
            if stdin.is_terminal() {
                return Err(Error::Input(
                    "o token vem pela entrada padrão, por exemplo \
                     `printf %s \"$TOKEN\" | garimpo sessao entregar`"
                        .into(),
                ));
            }
            let mut raw = String::new();
            stdin
                .read_to_string(&mut raw)
                .context("não consegui ler o token da entrada padrão")?;
            deliver(config, &Token::new(&raw)?)?;
            config.print("sessao guardada", &json!({"sessao": "guardada"}))?;
        }
        SessionCommand::Oauth {
            app_key,
            port,
            wait_s,
        } => {
            let token = oauth::login(config, app_key, *port, Duration::from_secs(*wait_s))?;
            deliver(config, &token)?;
            config.print("sessao guardada", &json!({"sessao": "guardada"}))?;
        }
        SessionCommand::Check => {
            let user = CurrentUser::fetch(&Transport::new(config))?;
            let line = format!(
                "sessao valida\tagente {}\tpessoa {}",
                crate::familysearch::field(user.tree_user_id.as_deref()),
                crate::familysearch::field(user.person_id.as_deref()),
            );
            config.print(
                &line,
                &json!({"sessao": "valida", "agente": user.tree_user_id, "pessoa": user.person_id}),
            )?;
        }
        SessionCommand::End => {
            end(&config.dir)?;
            config.print("sessao encerrada", &json!({"sessao": "encerrada"}))?;
        }
        SessionCommand::Hold => hold(&config.dir)?,
    }
    Ok(())
}

fn socket_path(dir: &Path) -> PathBuf {
    dir.join("session.sock")
}

/// Replaces the held session: ends the previous holder and starts another with
/// the new token.
fn deliver(config: &Config, token: &Token) -> Result<(), Error> {
    // Taking the account's turn first lets a request in flight with the old
    // session finish before the session is replaced.
    let turn = Turn::take(config)?;
    let dir = &config.dir;
    end(dir)?;
    // The holder outlives this command, so it is not waited for here. It gets
    // its own process group so it does not die with the agent's command, and it
    // does not inherit stderr so it does not keep the caller's pipe open.
    let mut holder = Command::new(env::current_exe().context("não achei o próprio executável")?)
        .arg("--dir")
        .arg(dir)
        .args(["sessao", "guardar"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .context("não consegui iniciar o guardião da sessão")?;
    // The token goes through the pipe, not an argument, to stay out of `ps`.
    let mut stdin = holder.stdin.take().expect("the holder's stdin is a pipe");
    writeln!(stdin, "{}", token.as_str()).context("não consegui passar o token ao guardião")?;
    drop(stdin);
    let stdout = holder.stdout.take().expect("the holder's stdout is a pipe");
    let mut answer = String::new();
    BufReader::new(stdout)
        .read_line(&mut answer)
        .context("não consegui ler a resposta do guardião")?;
    if answer.trim_end() != "pronto" {
        return Err(anyhow!("o guardião da sessão não iniciou: {}", answer.trim_end()).into());
    }
    turn.new_session()
}

/// The held token.
///
/// Returns `Error::SessionMissing` if there is no holder.
pub(crate) fn token(dir: &Path) -> Result<Token, Error> {
    let mut connection = match UnixStream::connect(socket_path(dir)) {
        Ok(connection) => connection,
        Err(error) => {
            return Err(match error.kind() {
                ErrorKind::NotFound | ErrorKind::ConnectionRefused => Error::SessionMissing,
                _ => anyhow!(error)
                    .context("não consegui falar com o guardião da sessão")
                    .into(),
            });
        }
    };
    let mut answer = String::new();
    connection
        .write_all(b"token\n")
        .and_then(|()| connection.read_to_string(&mut answer))
        .context("não consegui falar com o guardião da sessão")?;
    Token::new(&answer)
}

/// Drops the held session, if there is one.
pub(crate) fn end(dir: &Path) -> Result<(), Error> {
    tell_holder(dir, "end")?;
    // Left behind by a holder that died without cleaning up.
    let path = socket_path(dir);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(anyhow!(error)
            .context(format!("não consegui apagar {}", path.display()))
            .into()),
    }
}

/// Drops the held session only if it still is the one `token` came from, so a
/// request answered 401 with an old token cannot end a session delivered since.
pub(crate) fn expire(dir: &Path, token: &Token) -> Result<(), Error> {
    tell_holder(dir, &format!("expire {}", token.as_str()))
}

/// Sends one request to the holder, if there is one, and waits for it to close
/// the connection.
fn tell_holder(dir: &Path, request: &str) -> Result<(), Error> {
    if let Ok(mut connection) = UnixStream::connect(socket_path(dir)) {
        // A holder that ends removes the socket before closing the connection,
        // so reading to the end guarantees it is gone.
        connection
            .write_all(format!("{request}\n").as_bytes())
            .and_then(|()| connection.read_to_end(&mut Vec::new()))
            .context("o guardião da sessão não respondeu")?;
    }
    Ok(())
}

/// The holder process: reads the token from stdin and serves it over the
/// socket until it is told to end.
fn hold(dir: &Path) -> Result<(), Error> {
    let mut raw = String::new();
    io::stdin()
        .read_line(&mut raw)
        .context("não consegui ler o token")?;
    let token = Token::new(&raw)?;
    let path = socket_path(dir);
    let listener = UnixListener::bind(&path)
        // Only the owner talks to the holder, even if the state directory is open.
        .and_then(|listener| {
            fs::set_permissions(&path, Permissions::from_mode(0o600))?;
            Ok(listener)
        })
        .with_context(|| {
            format!(
                "não consegui abrir o soquete {} (o caminho de um soquete não passa de ~100 \
                 bytes; use um GARIMPO_DIR mais curto)",
                path.display()
            )
        });
    // Whoever delivered the session waits for this line to know how it went.
    let mut stdout = io::stdout();
    match &listener {
        Ok(_) => writeln!(stdout, "pronto"),
        Err(error) => writeln!(stdout, "{error:#}"),
    }
    .context("não consegui avisar quem entregou a sessão")?;
    for connection in listener?.incoming() {
        // A client that drops mid-conversation does not take the session down.
        let Ok(connection) = connection else { continue };
        if let Ok(Request::End) = serve(&connection, &token) {
            // Remove the socket before closing this connection: whoever asked to
            // end waits for the close to start another holder.
            fs::remove_file(&path)
                .with_context(|| format!("não consegui apagar {}", path.display()))?;
            break;
        }
    }
    Ok(())
}

/// What became of a client's request to the holder.
enum Request {
    /// The token was sent, or the request was not for this session: keep serving.
    Served,
    End,
}

/// Serves one request to the holder.
fn serve(mut connection: &UnixStream, token: &Token) -> io::Result<Request> {
    // A silent client must not hold up the holder, which serves one at a time.
    connection.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = String::new();
    BufReader::new(connection).read_line(&mut request)?;
    let request = request.trim_end();
    if request == "end" || request.strip_prefix("expire ") == Some(token.as_str()) {
        return Ok(Request::End);
    }
    if request == "token" {
        connection.write_all(token.as_str().as_bytes())?;
    }
    Ok(Request::Served)
}
