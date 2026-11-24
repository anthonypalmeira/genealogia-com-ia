//! The browser bridge: website requests made inside the user's logged tab.
//!
//! The website refuses its internal routes outside a browser. So a bridge
//! process listens on the loopback interface, and a loop evaluated once in the
//! logged tab asks it for work, makes each request from the page itself and
//! posts the answer back. The session never leaves the tab, and the loop uses
//! no timers, which a background tab would throttle.
//!
//! Commands reach the bridge over a Unix socket in the state directory, one at
//! a time: the account's turn already puts them in line.

use std::{
    env,
    fs::{self, Permissions},
    io::{self, BufRead, BufReader, ErrorKind, Read, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
        process::CommandExt,
    },
    panic,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
    thread::{self, ScopedJoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tiny_http::{Header, Method, Server};

use crate::{
    cli::{BridgeCommand, Config},
    error::Error,
    session::random_string,
};

/// How long the bridge holds the tab's request for work before answering
/// "nothing yet", which makes a live tab ask again.
const POLL_HOLD: Duration = Duration::from_secs(10);

/// How long the tab may go without asking for work and still count as
/// attached, and how long a request waits for the tab to take it. The tab asks
/// again right after each answer and accepts a request at once, so the gap is
/// short.
const TAB_GRACE: Duration = Duration::from_secs(3);

/// How long the tab has to answer a request it took: the same time a direct
/// request has.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(60);

/// What a command asks of the bridge, as one line of JSON over the socket.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    /// Make a GET to a path of the website.
    Fetch {
        path: String,
    },
    /// Tell whether a tab is attached.
    Status,
    Close,
}

/// What the bridge answers a command, as one line of JSON.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Reply {
    /// Whether a tab is attached: the answer to `Request::Status`.
    Tab(bool),
    /// No tab took the request, so it was not made.
    NoTab,
    /// The tab took the request and did not answer in time.
    NoAnswer,
    /// The tab could not make the request.
    Error(String),
    Answer(Answer),
}

/// What the website answered a request made in the tab: the status, the
/// headers the regulator reads, as text, and the body.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Answer {
    pub(crate) status: u16,
    pub(crate) retry_after: Option<String>,
    pub(crate) processing_time: Option<String>,
    pub(crate) location: Option<String>,
    /// `None` when the tab did not read the body: only a successful answer
    /// has one worth waiting for, and even that one may fail to arrive.
    pub(crate) body: Option<String>,
}

/// What the tab posts when it is done with a request.
#[derive(Debug, Deserialize)]
struct Done {
    id: u64,
    #[serde(flatten)]
    outcome: Outcome,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Answer(Answer),
    Error(String),
}

/// Runs a command of the `ponte` group.
pub(crate) fn run(config: &Config, command: &BridgeCommand) -> Result<(), Error> {
    match command {
        BridgeCommand::Open => {
            let address = open(config)?;
            let snippet = snippet(&address, &config.fs_site);
            config.print(
                &snippet,
                &json!({"ponte": "aberta", "endereco": address, "trecho": snippet}),
            )?;
        }
        BridgeCommand::Status => match ask(&config.dir, &Request::Status)? {
            Some(Reply::Tab(true)) => config.print("ponte ligada", &json!({"ponte": "ligada"}))?,
            // No bridge at all is no tab either.
            None
            | Some(
                Reply::Tab(false)
                | Reply::NoTab
                | Reply::NoAnswer
                | Reply::Error(_)
                | Reply::Answer(_),
            ) => return Err(Error::BridgeDetached),
        },
        BridgeCommand::Close => {
            close(&config.dir)?;
            config.print("ponte fechada", &json!({"ponte": "fechada"}))?;
        }
        BridgeCommand::Serve => serve(config)?,
    }
    Ok(())
}

fn socket_path(dir: &Path) -> PathBuf {
    dir.join("bridge.sock")
}

/// Whether a bridge is open for this account.
#[must_use = "the answer decides which way a request goes"]
pub(crate) fn is_open(dir: &Path) -> bool {
    // A connection that says nothing is ignored by the bridge.
    UnixStream::connect(socket_path(dir)).is_ok()
}

/// Makes a GET to `path` of the website inside the logged tab.
///
/// Returns `Error::BridgeDetached` when no tab is running the bridge's loop.
pub(crate) fn fetch(dir: &Path, path: &str) -> Result<Answer, Error> {
    let request = Request::Fetch {
        path: path.to_owned(),
    };
    match ask(dir, &request)? {
        // No reply at all: the bridge closed since the caller saw it open.
        None | Some(Reply::NoTab) => Err(Error::BridgeDetached),
        Some(Reply::Answer(answer)) => Ok(answer),
        Some(Reply::NoAnswer) => {
            Err(anyhow!("a aba não respondeu a tempo ao pedido GET {path}").into())
        }
        Some(Reply::Error(error)) => {
            Err(anyhow!("a aba não completou o pedido GET {path}: {error}").into())
        }
        Some(Reply::Tab(_)) => Err(anyhow!("a ponte respondeu algo inesperado").into()),
    }
}

/// Sends one request to the bridge and reads its reply. `None` when no bridge
/// is open, or when the bridge closes instead of replying.
fn ask(dir: &Path, request: &Request) -> Result<Option<Reply>, Error> {
    let mut connection = match UnixStream::connect(socket_path(dir)) {
        Ok(connection) => connection,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::NotFound | ErrorKind::ConnectionRefused
            ) =>
        {
            return Ok(None);
        }
        Err(error) => {
            return Err(anyhow!(error)
                .context("não consegui falar com a ponte")
                .into());
        }
    };
    let line = serde_json::to_string(request).context("não consegui gerar o pedido à ponte")?;
    let mut reply = String::new();
    connection
        .write_all(format!("{line}\n").as_bytes())
        .and_then(|()| connection.read_to_string(&mut reply))
        .context("a ponte não respondeu")?;
    if reply.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        serde_json::from_str(&reply).context("a ponte respondeu algo inesperado")?,
    ))
}

/// Closes the bridge, if one is open.
fn close(dir: &Path) -> Result<(), Error> {
    // A bridge that closes only ends this connection once it is gone.
    ask(dir, &Request::Close)?;
    // Left behind by a bridge that died without cleaning up.
    let path = socket_path(dir);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(anyhow!(error)
            .context(format!("não consegui apagar {}", path.display()))
            .into()),
    }
}

/// Replaces the bridge: closes the previous one and starts another. Returns
/// the address the tab talks to, which carries the bridge's secret.
fn open(config: &Config) -> Result<String, Error> {
    let dir = config.state_dir()?;
    close(dir)?;
    // The bridge outlives this command, so it is not waited for here. It gets
    // its own process group so it does not die with the agent's command, and it
    // does not inherit stderr so it does not keep the caller's pipe open.
    let mut bridge = Command::new(env::current_exe().context("não achei o próprio executável")?)
        .arg("--dir")
        .arg(dir)
        .arg("--fs-site")
        .arg(&config.fs_site)
        .args(["ponte", "servir"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .context("não consegui iniciar a ponte")?;
    let stdout = bridge.stdout.take().expect("the bridge's stdout is a pipe");
    let mut answer = String::new();
    BufReader::new(stdout)
        .read_line(&mut answer)
        .context("não consegui ler a resposta da ponte")?;
    match answer.trim_end().strip_prefix("pronto ") {
        Some(address) => Ok(address.to_owned()),
        None => Err(anyhow!("a ponte não abriu: {}", answer.trim_end()).into()),
    }
}

/// The loop the agent evaluates once in the logged tab, on one line. It
/// returns at once and keeps serving in the background: it asks the bridge for
/// work, accepts it, makes the request from the page with the page's own
/// session, and posts the answer. It ends when the bridge closes or the page
/// goes away.
fn snippet(address: &str, site: &str) -> String {
    // A JSON string is a JavaScript string literal.
    let literal = |text: &str| Value::from(text).to_string();
    include_str!("bridge/tab.js")
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
        .replace("\"__BRIDGE__\"", &literal(address))
        .replace("\"__SITE__\"", &literal(site))
}

/// What reaches the side of the bridge that talks to the tab.
enum Event {
    /// The tab asks for work.
    Poll(tiny_http::Request),
    /// The tab asks to accept the request with this id, before making it.
    Taken {
        id: u64,
        request: tiny_http::Request,
    },
    /// The tab is done with a request.
    Done(Done),
    /// A command asks for a request to be made.
    Fetch {
        path: String,
        reply: SyncSender<Reply>,
    },
    /// A command asks whether a tab is attached.
    Status {
        reply: SyncSender<Reply>,
    },
    Close,
}

/// The two doors of the bridge: HTTP on the loopback interface for the tab,
/// the socket for the commands.
struct Doors {
    http: Server,
    /// The address the tab talks to: the port and the secret that keeps other
    /// pages and programs of this machine from posing as the tab.
    address: String,
    /// What the path of every request of the tab starts with: the secret.
    prefix: String,
    cors: Vec<Header>,
    commands: UnixListener,
}

impl Doors {
    fn open(config: &Config, socket: &Path) -> Result<Self, Error> {
        let cors = cors(&config.fs_site)?;
        let secret = random_string()?;
        // Port 0: the system picks a free one.
        let http = Server::http("127.0.0.1:0")
            .map_err(|error| anyhow!("não consegui abrir uma porta em 127.0.0.1: {error}"))?;
        let port = http
            .server_addr()
            .to_ip()
            .context("a ponte não abriu numa porta de rede")?
            .port();
        let commands = UnixListener::bind(socket)
            // Only the owner talks to the bridge, even if the state directory is open.
            .and_then(|listener| {
                fs::set_permissions(socket, Permissions::from_mode(0o600))?;
                Ok(listener)
            })
            .with_context(|| {
                format!(
                    "não consegui abrir o soquete {} (o caminho de um soquete não passa de \
                     ~100 bytes; use um GARIMPO_DIR mais curto)",
                    socket.display()
                )
            })?;
        Ok(Self {
            http,
            address: format!("http://127.0.0.1:{port}/{secret}"),
            prefix: format!("/{secret}/"),
            cors,
            commands,
        })
    }
}

/// The bridge process: serves the tab and the commands until it is told to
/// close.
fn serve(config: &Config) -> Result<(), Error> {
    let socket = socket_path(&config.dir);
    let doors = Doors::open(config, &socket);
    // Whoever opened the bridge waits for this line to know how it went.
    let mut stdout = io::stdout();
    match &doors {
        Ok(doors) => writeln!(stdout, "pronto {}", doors.address),
        Err(error) => writeln!(stdout, "{error:#}"),
    }
    .context("não consegui avisar quem abriu a ponte")?;
    let Doors {
        http,
        prefix,
        cors,
        commands,
        ..
    } = doors?;
    let (events, inbox) = mpsc::sync_channel(8);
    let closer = thread::scope(|scope| {
        let named = |name: &str| thread::Builder::new().name(name.to_owned());
        let from_tab =
            named("ponte-aba").spawn_scoped(scope, || feed(&http, &prefix, &cors, &events))?;
        let from_commands =
            named("ponte-comandos").spawn_scoped(scope, || attend(&commands, &events));
        match &from_commands {
            Ok(_) => Tab::new(&cors).run(inbox),
            // Without its thread no command could ever close the bridge. With
            // the inbox gone, the other thread is not left waiting to send.
            Err(_) => drop(inbox),
        }
        http.unblock();
        joined(from_tab);
        from_commands.map(joined)
    });
    // Whoever asked to close waits for the end of its connection to open
    // another bridge, so the port and the socket are released first.
    drop(http);
    fs::remove_file(&socket)
        .with_context(|| format!("não consegui apagar {}", socket.display()))?;
    drop(closer.context("não consegui iniciar as linhas de execução da ponte")?);
    Ok(())
}

/// Waits for a thread of the bridge; its panic becomes this thread's.
fn joined<T>(thread: ScopedJoinHandle<'_, T>) -> T {
    thread
        .join()
        .unwrap_or_else(|panicked| panic::resume_unwind(panicked))
}

/// The headers that let the page at `origin`, and only it, talk to the bridge.
fn cors(origin: &str) -> Result<Vec<Header>, Error> {
    [
        ("Access-Control-Allow-Origin", origin),
        ("Access-Control-Allow-Methods", "GET, POST"),
        // A browser may first ask whether a public page can reach a local server.
        ("Access-Control-Allow-Private-Network", "true"),
        ("Cache-Control", "no-store"),
    ]
    .into_iter()
    .map(|(name, value)| {
        Header::from_bytes(name, value)
            .map_err(|()| Error::Input(format!("--fs-site não serve como origem da aba: {value}")))
    })
    .collect()
}

/// Answers the tab.
fn respond(request: tiny_http::Request, cors: &[Header], status: u16, body: String) {
    let mut response = tiny_http::Response::from_string(body).with_status_code(status);
    for header in cors {
        response.add_header(header.clone());
    }
    // A failed write means the tab is gone, which the bridge finds out when
    // the tab stops asking for work.
    let _ = request.respond(response);
}

/// Receives the tab's HTTP requests and passes on what they mean, until the
/// server is unblocked.
fn feed(http: &Server, prefix: &str, cors: &[Header], events: &SyncSender<Event>) {
    for mut request in http.incoming_requests() {
        let method = request.method().clone();
        let route = request.url().strip_prefix(prefix).map(str::to_owned);
        let event = match (method, route.as_deref()) {
            (Method::Get, Some("proxima")) => Some(Event::Poll(request)),
            (Method::Post, Some(posted @ ("aceito" | "resposta"))) => {
                let mut body = String::new();
                let read = request.as_reader().read_to_string(&mut body).is_ok();
                let id = body
                    .trim()
                    .parse()
                    .ok()
                    .filter(|_| read && posted == "aceito");
                let done = serde_json::from_str(&body)
                    .ok()
                    .filter(|_| read && posted == "resposta");
                match (id, done) {
                    // The side that holds the request decides whether it still stands.
                    (Some(id), _) => Some(Event::Taken { id, request }),
                    (None, Some(done)) => {
                        respond(request, cors, 204, String::new());
                        Some(Event::Done(done))
                    }
                    (None, None) => {
                        respond(request, cors, 400, String::new());
                        None
                    }
                }
            }
            (Method::Options, _) => {
                respond(request, cors, 204, String::new());
                None
            }
            // Without the secret there is nothing here.
            _ => {
                respond(request, cors, 404, String::new());
                None
            }
        };
        if let Some(event) = event
            && events.send(event).is_err()
        {
            break;
        }
    }
}

/// Serves the commands, one connection at a time, until one asks to close.
/// Returns that connection, still open.
fn attend(listener: &UnixListener, events: &SyncSender<Event>) -> Option<UnixStream> {
    for connection in listener.incoming() {
        // A client that drops mid-conversation does not take the bridge down.
        let Ok(connection) = connection else { continue };
        let Ok(Some(request)) = read_request(&connection) else {
            continue;
        };
        let (reply, answer) = mpsc::sync_channel(1);
        let event = match request {
            Request::Fetch { path } => Event::Fetch { path, reply },
            Request::Status => Event::Status { reply },
            Request::Close => {
                // If the other side is already gone, the bridge is closing anyway.
                let _ = events.send(Event::Close);
                return Some(connection);
            }
        };
        if events.send(event).is_err() {
            return None;
        }
        // The side that talks to the tab always answers a request it took.
        if let Ok(reply) = answer.recv()
            && let Ok(line) = serde_json::to_string(&reply)
        {
            // The command may have given up waiting.
            let _ = (&connection).write_all(format!("{line}\n").as_bytes());
        }
    }
    None
}

/// Reads the request of one connection. `None` for a connection that only
/// checked whether the bridge is open, or sent something else.
fn read_request(connection: &UnixStream) -> io::Result<Option<Request>> {
    // A silent client must not hold up the bridge, which serves one at a time.
    connection.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut line = String::new();
    BufReader::new(connection).read_line(&mut line)?;
    Ok(serde_json::from_str(&line).ok())
}

/// How far the one request in flight went.
#[derive(Clone, Copy)]
enum Stage {
    /// Waiting for the tab to ask for work, since this moment.
    Waiting(Instant),
    /// Sent to the tab at this moment, and not accepted yet. A tab that went
    /// away may have left its request for work behind, so being sent is not
    /// being taken.
    Offered(Instant),
    /// Accepted by the tab at this moment, with the bridge's leave: the
    /// request is being made.
    Taken(Instant),
}

/// The one request in flight. Commands come one at a time.
struct Job {
    id: u64,
    /// What the tab is asked to do, as JSON.
    work: String,
    reply: SyncSender<Reply>,
    stage: Stage,
}

impl Job {
    /// When the job has waited too long in its stage.
    fn deadline(&self) -> Instant {
        match self.stage {
            Stage::Waiting(since) | Stage::Offered(since) => since + TAB_GRACE,
            Stage::Taken(since) => since + ANSWER_TIMEOUT,
        }
    }

    /// Hands the command its reply.
    fn answer(&self, reply: Reply) {
        // The command may have given up waiting.
        let _ = self.reply.send(reply);
    }
}

/// The side of the bridge that talks to the tab. It alone holds the state,
/// and everything reaches it as an event.
struct Tab<'a> {
    cors: &'a [Header],
    /// The tab's request for work, held until there is some or it gets old.
    parked: Option<(tiny_http::Request, Instant)>,
    /// When the tab last showed up.
    seen: Option<Instant>,
    job: Option<Job>,
    /// The id of the last job, to number the next one.
    last_id: u64,
}

impl<'a> Tab<'a> {
    fn new(cors: &'a [Header]) -> Self {
        Self {
            cors,
            parked: None,
            seen: None,
            job: None,
            last_id: 0,
        }
    }

    /// Handles events until the bridge is told to close. Takes the inbox with
    /// it, so nobody is left waiting to send once it stops.
    fn run(mut self, inbox: Receiver<Event>) {
        loop {
            // With nothing waiting on the tab, the bridge sleeps until an event.
            let poll = self.parked.as_ref().map(|(_, since)| *since + POLL_HOLD);
            let job = self.job.as_ref().map(Job::deadline);
            let event = match poll.into_iter().chain(job).min() {
                Some(deadline) => {
                    inbox.recv_timeout(deadline.saturating_duration_since(Instant::now()))
                }
                None => inbox.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            let now = Instant::now();
            match event {
                Ok(Event::Poll(request)) => {
                    self.seen = Some(now);
                    if let Some((stale, _)) = self.parked.replace((request, now)) {
                        respond(stale, self.cors, 204, String::new());
                    }
                    self.offer(now);
                }
                Ok(Event::Taken { id, request }) => {
                    self.seen = Some(now);
                    // A request that already got its reply must not be made: the
                    // command is gone and nobody would count it.
                    let job = self.job.as_mut().filter(|job| job.id == id);
                    let status = if job.is_some() { 204 } else { 409 };
                    if let Some(job) = job {
                        job.stage = Stage::Taken(now);
                    }
                    respond(request, self.cors, status, String::new());
                }
                Ok(Event::Done(done)) => {
                    self.seen = Some(now);
                    // An answer that comes after its deadline has nobody waiting.
                    if let Some(job) = self.job.take_if(|job| job.id == done.id) {
                        job.answer(match done.outcome {
                            Outcome::Answer(answer) => Reply::Answer(answer),
                            Outcome::Error(error) => Reply::Error(error),
                        });
                    }
                }
                Ok(Event::Fetch { path, reply }) => {
                    self.last_id += 1;
                    self.job = Some(Job {
                        id: self.last_id,
                        work: json!({"id": self.last_id, "path": path}).to_string(),
                        reply,
                        stage: Stage::Waiting(now),
                    });
                    self.offer(now);
                }
                Ok(Event::Status { reply }) => {
                    // Whoever asked may have given up waiting.
                    let _ = reply.send(Reply::Tab(self.attached(now)));
                }
                Ok(Event::Close) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.expire(now);
        }
        // The tab's loop ends when its request for work is refused.
        if let Some((request, _)) = self.parked.take() {
            respond(request, self.cors, 410, String::new());
        }
    }

    /// Whether a tab is running the loop: it is waiting for work, or it showed
    /// up a moment ago and is about to ask again.
    fn attached(&self, now: Instant) -> bool {
        self.parked.is_some() || self.seen.is_some_and(|seen| now < seen + TAB_GRACE)
    }

    /// Sends the waiting job to the tab, if the tab is asking for work.
    fn offer(&mut self, now: Instant) {
        if let Some(job) = self.job.as_mut()
            && let Stage::Waiting(_) = job.stage
            && let Some((request, _)) = self.parked.take()
        {
            respond(request, self.cors, 200, job.work.clone());
            job.stage = Stage::Offered(now);
        }
    }

    /// Ends what waited too long: the tab's request for work, so a live tab
    /// asks again, and a job that no tab took or answered in time.
    fn expire(&mut self, now: Instant) {
        if let Some((request, _)) = self.parked.take_if(|(_, since)| now >= *since + POLL_HOLD) {
            respond(request, self.cors, 204, String::new());
            self.seen = Some(now);
        }
        if let Some(job) = self.job.take_if(|job| now >= job.deadline()) {
            job.answer(match job.stage {
                Stage::Waiting(_) | Stage::Offered(_) => Reply::NoTab,
                Stage::Taken(_) => Reply::NoAnswer,
            });
        }
    }
}
