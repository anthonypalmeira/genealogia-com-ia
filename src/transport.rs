//! The only way to FamilySearch: HTTP requests with the held session, at the
//! account's pace and with no disguise.

use std::{
    cell::OnceCell,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, anyhow};
use serde_json::Value;
use ureq::{Agent, config::Config as HttpConfig, http::HeaderMap};

use crate::{
    cli::Config,
    error::Error,
    pacing::{Turn, millis},
    session::{self, Token},
};

/// The media type of the tree routes, with the FamilySearch extensions.
const MEDIA_TYPE: &str = "application/x-fs-v1+json";

/// How many times a request is made while the server answers 429.
const ATTEMPTS: u32 = 3;

/// A successful response.
#[derive(Debug)]
pub(crate) struct Response {
    /// What a GET read; empty on a 204 and for a POST.
    pub(crate) body: String,
    /// The `Location` header: the URL of what a POST created.
    pub(crate) location: Option<String>,
}

/// Authenticated requests to the FamilySearch base URL.
#[derive(Debug)]
pub(crate) struct Transport<'a> {
    config: &'a Config,
    agent: Agent,
    /// The session of the first request. One command never spans two sessions.
    token: OnceCell<Token>,
}

impl<'a> Transport<'a> {
    pub(crate) fn new(config: &'a Config) -> Self {
        Self {
            config,
            agent: agent(),
            token: OnceCell::new(),
        }
    }

    pub(crate) fn get(&self, path: &str) -> Result<Response, Error> {
        self.send(path, None)
    }

    pub(crate) fn post(&self, path: &str, body: &Value) -> Result<Response, Error> {
        self.send(path, Some(body))
    }

    /// Makes one request at the account's pace: waits for the turn, the pause
    /// and the window budget, and repeats after the pause when the server
    /// answers 429 (a refusal before processing, so repeating a POST duplicates
    /// nothing). A 403 is a block on the first answer, with no retry.
    fn send(&self, path: &str, body: Option<&Value>) -> Result<Response, Error> {
        let method = if body.is_some() { "POST" } else { "GET" };
        let url = format!("{}{path}", self.config.fs_api);
        for _ in 0..ATTEMPTS {
            // The turn lasts until the response is consumed or dropped.
            let mut turn = Turn::take(self.config)?;
            // The token is read inside the turn, so a command that waited in line
            // sees the session that is current when its request would go out. If
            // that is no longer the session the command started with, it stops:
            // a check made in one session must not be followed by a write in another.
            let current = session::token(&self.config.dir)?;
            let token = match self.token.get() {
                None => self.token.get_or_init(|| current),
                Some(first) if *first == current => first,
                Some(_) => {
                    return Err(anyhow!("a sessão foi trocada durante o comando").into());
                }
            };
            let authorization = format!("Bearer {}", token.as_str());
            let wait = turn.wait(self.config, now_ms()?)?;
            if wait > Duration::from_secs(self.config.max_wait_s) {
                return Err(Error::Paused {
                    seconds: wait.as_secs(),
                });
            }
            if wait >= Duration::from_secs(1) {
                eprintln!(
                    "garimpo: aguardando {} s para respeitar o ritmo do FamilySearch",
                    wait.as_secs()
                );
            }
            thread::sleep(wait);
            let started = Instant::now();
            let sent = match body {
                None => self
                    .agent
                    .get(&url)
                    .header("Authorization", &authorization)
                    .header("Accept", MEDIA_TYPE)
                    .call(),
                Some(body) => self
                    .agent
                    .post(&url)
                    .header("Authorization", &authorization)
                    .header("Accept", MEDIA_TYPE)
                    .content_type(MEDIA_TYPE)
                    .send(body.to_string()),
            };
            let mut response = match sent {
                Ok(response) => response,
                Err(error) => {
                    // The request may have reached the server, so it still counts.
                    turn.record(self.config, now_ms()?, millis(started.elapsed()), None)?;
                    return Err(anyhow!(error)
                        .context(format!("o pedido {method} {path} falhou"))
                        .into());
                }
            };
            let status = response.status().as_u16();
            let headers = response.headers();
            // Without a reported processing time, the time the request took counts.
            let cost_ms = header_number(headers, "x-processing-time")
                .unwrap_or_else(|| millis(started.elapsed()));
            // The server asks for a pause with `Retry-After`, in seconds. After a
            // 429 the pause is long, even when the server asks for little.
            let asked = header_number(headers, "retry-after").map(Duration::from_secs);
            let pause = if status == 429 {
                let minimum = Duration::from_secs(self.config.pause_429_s);
                Some(asked.unwrap_or_default().max(minimum))
            } else {
                asked
            };
            let location = headers
                .get("location")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            // The count and the pause are saved from the headers alone, before any
            // body is read, so a body that stalls or breaks cannot lose them. The
            // turn itself goes on while the body is read.
            turn.record(self.config, now_ms()?, cost_ms, pause)?;
            match status {
                200..=299 => {
                    // Only what a GET read is used; a POST is told by its status
                    // and `Location`.
                    let text = if body.is_none() {
                        response.body_mut().read_to_string().with_context(|| {
                            format!("não consegui ler a resposta de {method} {path}")
                        })?
                    } else {
                        String::new()
                    };
                    return Ok(Response {
                        body: text,
                        location,
                    });
                }
                // Redirects are not followed: a hidden second request would skip
                // the regulator. A merged person answers with one.
                300..=399 => {
                    return Err(Error::Input(format!(
                        "{path} mudou para {} ({status}); a pessoa pode ter sido fundida em \
                         outra: repita com o id novo",
                        location.as_deref().unwrap_or("outro endereço")
                    )));
                }
                401 => {
                    // The session is dead: drop it, so the next command does not
                    // even try. A newer session delivered meanwhile is left alone.
                    session::expire(&self.config.dir, token)?;
                    return Err(Error::SessionExpired);
                }
                403 => {
                    return Err(Error::Blocked {
                        status,
                        path: path.to_owned(),
                    });
                }
                404 | 410 => {
                    return Err(Error::Input(format!(
                        "o FamilySearch não achou {path} ({status})"
                    )));
                }
                429 => eprintln!(
                    "garimpo: o FamilySearch respondeu 429; pausa de {} s",
                    pause.unwrap_or_default().as_secs()
                ),
                _ => {
                    return Err(
                        anyhow!("o FamilySearch respondeu {status} em {method} {path}").into(),
                    );
                }
            }
        }
        Err(Error::Blocked {
            status: 429,
            path: path.to_owned(),
        })
    }
}

/// The HTTP client of `garimpo`. Error responses arrive as responses, not as
/// failures, so the caller can read the status and the headers.
#[must_use = "the client is only useful to make requests"]
pub(crate) fn agent() -> Agent {
    HttpConfig::builder()
        .http_status_as_error(false)
        // Honest identification: no posing as a browser.
        .user_agent(concat!("garimpo/", env!("CARGO_PKG_VERSION")))
        // Every request goes through the regulator, so redirects are not followed.
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .new_agent()
}

fn now_ms() -> Result<u64, Error> {
    let since_1970 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("o relógio do sistema está antes de 1970")?;
    Ok(millis(since_1970))
}

/// A header whose value is a whole number, such as `Retry-After` in seconds.
fn header_number(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers.get(name)?.to_str().ok()?.trim().parse().ok()
}
