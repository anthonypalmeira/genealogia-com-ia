//! Login through the official API: OAuth 2 with authorization code, PKCE and a
//! loopback redirect, for whoever has a FamilySearch app key.

use std::{
    io::{self, BufRead, BufReader, ErrorKind, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{Token, random_string};
use crate::{cli::Config, error::Error, transport};

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Logs in: prints the URL the user opens in the browser, waits for the
/// redirect on `127.0.0.1:port` and exchanges the code for the token.
pub(super) fn login(
    config: &Config,
    app_key: &str,
    port: u16,
    limit: Duration,
) -> Result<Token, Error> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("não consegui ouvir o retorno em 127.0.0.1:{port}"))?;
    let port = listener
        .local_addr()
        .context("não consegui ler a porta do retorno")?
        .port();
    let redirect = format!("http://127.0.0.1:{port}");
    let verifier = random_string()?;
    let state = random_string()?;
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("response_type", "code")
        .append_pair("client_id", app_key)
        .append_pair("redirect_uri", &redirect)
        .append_pair("scope", "openid")
        .append_pair("state", &state)
        .append_pair(
            "code_challenge",
            &URL_SAFE_NO_PAD.encode(Sha256::digest(&verifier)),
        )
        .append_pair("code_challenge_method", "S256")
        .finish();
    let authorization = format!(
        "{}/cis-web/oauth2/v3/authorization?{query}",
        config.fs_ident
    );
    config.print(&authorization, &json!({"abrir": authorization}))?;

    let code = await_callback(&listener, &state, limit)?;
    let mut response = transport::agent()
        .post(format!("{}/cis-web/oauth2/v3/token", config.fs_ident))
        .send_form([
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("client_id", app_key),
            ("redirect_uri", &redirect),
            ("code_verifier", &verifier),
        ])
        .context("a troca do código pelo token falhou")?;
    let status = response.status();
    if !status.is_success() {
        return Err(
            anyhow!("o FamilySearch respondeu {status} na troca do código pelo token").into(),
        );
    }
    let body: TokenResponse = response
        .body_mut()
        .read_json()
        .context("a resposta da troca do código não trouxe o token")?;
    Token::new(&body.access_token)
}

/// What a request to the loopback redirect brought.
enum Callback {
    Code(String),
    /// FamilySearch refused the login; carries the reason.
    Refused(String),
    /// A request that is not this login's redirect.
    Other,
}

/// Waits for the browser to come back with the authorization code.
fn await_callback(listener: &TcpListener, state: &str, limit: Duration) -> Result<String, Error> {
    // `accept` has no deadline, so the listener does not block and is checked
    // every 100 ms until the limit.
    listener
        .set_nonblocking(true)
        .context("não consegui preparar o retorno local")?;
    let deadline = Instant::now() + limit;
    loop {
        match listener.accept() {
            Ok((connection, _)) => match answer(&connection, state) {
                Ok(Callback::Code(code)) => return Ok(code),
                Ok(Callback::Refused(reason)) => {
                    return Err(anyhow!("o FamilySearch recusou o login: {reason}").into());
                }
                // A connection that is not the redirect, or that dropped (browsers
                // open spare connections), does not end the login.
                Ok(Callback::Other) | Err(_) => {}
            },
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    eprintln!("garimpo: o login não chegou em {} s", limit.as_secs());
                    return Err(Error::SessionMissing);
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(anyhow!(error).context("o retorno local falhou").into()),
        }
    }
}

/// Reads one browser request to the loopback redirect and answers it. Only a
/// request carrying the `state` sent in the authorization counts as this
/// login's redirect.
fn answer(mut connection: &TcpStream, state: &str) -> io::Result<Callback> {
    // On some systems the accepted connection inherits the non-blocking mode.
    connection.set_nonblocking(false)?;
    connection.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut lines = BufReader::new(connection).lines();
    let first = lines.next().transpose()?.unwrap_or_default();
    // Consume the headers, so the browser receives the whole response.
    for line in lines {
        if line?.is_empty() {
            break;
        }
    }
    let target = first.split_whitespace().nth(1).unwrap_or_default();
    let query = target.split_once('?').map_or("", |(_, query)| query);
    let parameter = |name: &str| {
        form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    let this_login = parameter("state").as_deref() == Some(state);
    let (status, text, callback) = match (this_login, parameter("code"), parameter("error")) {
        (true, Some(code), _) => (
            "200 OK",
            "Login recebido. Pode fechar esta aba e voltar ao agente.",
            Callback::Code(code),
        ),
        (true, None, Some(error)) => (
            "400 Bad Request",
            "O FamilySearch recusou o login. Volte ao agente.",
            Callback::Refused(format!(
                "{error} {}",
                parameter("error_description").unwrap_or_default()
            )),
        ),
        _ => ("404 Not Found", "Nada aqui.", Callback::Other),
    };
    write!(
        connection,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{text}",
        text.len()
    )?;
    Ok(callback)
}
