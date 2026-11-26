//! The only way to FamilySearch: HTTP requests at the account's pace and with
//! no disguise, made directly with the held session or, for the website's
//! routes, inside the user's logged tab through the browser bridge.

use std::{
    fs::File,
    io,
    path::Path,
    sync::OnceLock,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, anyhow};
use serde_json::Value;
use ureq::{Agent, config::Config as HttpConfig};

use crate::{
    bridge,
    cli::Config,
    error::{Error, Service},
    pacing::{Turn, millis},
    session::{self, Token},
};

/// The media type of the tree routes, with the FamilySearch extensions.
const MEDIA_TYPE: &str = "application/x-fs-v1+json";

/// The media type the website's own routes answer in.
const SITE_MEDIA_TYPE: &str = "application/json";

/// How many times a request is made while the server answers 429.
const ATTEMPTS: u32 = 3;

/// What a request is: where it goes and what it carries.
#[derive(Debug, Clone, Copy)]
enum Call<'a> {
    /// A GET to the official API.
    Get,
    /// A POST to the official API.
    Post(&'a Value),
    /// A GET to the official API for the portrait of a person, answered with
    /// a redirect to the image.
    Portrait,
    /// A GET to the website, made inside the logged tab while the browser
    /// bridge is open.
    SiteGet,
    /// A GET to the image storage, answered in JSON.
    StorageGet,
    /// A GET to the image storage for an image, whose bytes go into the file.
    Download(&'a Path),
}

impl Call<'_> {
    fn method(self) -> &'static str {
        match self {
            Self::Get | Self::Portrait | Self::SiteGet | Self::StorageGet | Self::Download(_) => {
                "GET"
            }
            Self::Post(_) => "POST",
        }
    }
}

/// Which way a request goes out.
#[derive(Debug, Clone, Copy)]
enum Way<'a> {
    /// From this process, with the held session.
    Direct(&'a Token),
    /// From the logged tab, which authenticates the request itself: the held
    /// session is neither needed nor read.
    Tab,
}

/// What one HTTP exchange brought, whichever way it went.
#[derive(Debug)]
struct Exchange {
    status: u16,
    /// `X-PROCESSING-TIME`: the processing time the server reports, in
    /// milliseconds.
    processing_ms: Option<u64>,
    /// `Retry-After`: the pause the server asks for, in seconds.
    retry_after_s: Option<u64>,
    location: Option<String>,
    body: Body,
}

/// The body of an exchange.
#[derive(Debug)]
enum Body {
    /// Still on the wire: read only when the status calls for it.
    Unread(ureq::Body),
    /// Already read, by the tab.
    Read(String),
    /// The tab could not read it.
    Lost,
}

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
    token: OnceLock<Token>,
    /// A session to check before it is held: used instead of the held one.
    unheld: Option<&'a Token>,
}

impl<'a> Transport<'a> {
    pub(crate) fn new(config: &'a Config) -> Self {
        Self {
            config,
            agent: agent(),
            token: OnceLock::new(),
            unheld: None,
        }
    }

    /// Requests made with `token`, a session not held yet, to check it. A 401
    /// leaves the held session alone, since it is not that one.
    pub(crate) fn with_session(config: &'a Config, token: &'a Token) -> Self {
        Self {
            unheld: Some(token),
            ..Self::new(config)
        }
    }

    pub(crate) fn get(&self, path: &str) -> Result<Response, Error> {
        self.send(Call::Get, path)
    }

    pub(crate) fn post(&self, path: &str, body: &Value) -> Result<Response, Error> {
        self.send(Call::Post(body), path)
    }

    /// A GET to the website instead of the API. With the browser bridge open
    /// it is made inside the logged tab; otherwise directly, like any other.
    pub(crate) fn get_site(&self, path: &str) -> Result<Response, Error> {
        self.send(Call::SiteGet, path)
    }

    /// A GET to the image storage that answers in JSON.
    pub(crate) fn get_storage(&self, path: &str) -> Result<Response, Error> {
        self.send(Call::StorageGet, path)
    }

    /// Asks the image storage for an image, at the account's pace like any
    /// other request. A thumbnail comes in the answer and goes into `into`. A
    /// full image is answered with a redirect to a signed address of the
    /// storage, which is returned for [`Self::transfer`]: that one request
    /// goes outside the account's turn, so the transfers of several threads
    /// overlap while the requests to FamilySearch stay one at a time.
    pub(crate) fn ask_image(&self, path: &str, into: &Path) -> Result<Option<String>, Error> {
        Ok(self.send(Call::Download(into), path)?.location)
    }

    /// Asks for the portrait of a person. The answer is the address of the
    /// image, for [`Self::transfer`], or `None` when the person has none.
    pub(crate) fn ask_portrait(&self, path: &str) -> Result<Option<String>, Error> {
        Ok(self.send(Call::Portrait, path)?.location)
    }

    /// Stores what the address of an image holds in `into`: the signed
    /// address of a film image or the fixed address of a portrait. Neither
    /// request carries the session or includes the address in an error.
    pub(crate) fn transfer(&self, address: &str, into: &Path) -> Result<(), Error> {
        let mut response = self.agent.get(address).call().map_err(|error| {
            let cause = match error {
                ureq::Error::Timeout(_) => "o tempo esgotou".to_owned(),
                ureq::Error::Io(error) => error.to_string(),
                _ => "a conexão não se completou".to_owned(),
            };
            anyhow!("a transferência da imagem falhou: {cause}")
        })?;
        let status = response.status().as_u16();
        if !(200..=299).contains(&status) {
            let retry_after = response.headers().get("retry-after");
            return Err(Error::ImageStorage {
                status,
                retry_after: retry_after
                    .and_then(|value| value.to_str().ok())
                    .map(|value| value.trim().to_owned()),
            });
        }
        File::create(into)
            .and_then(|mut file| io::copy(&mut response.body_mut().as_reader(), &mut file))
            .with_context(|| format!("não consegui gravar {}", into.display()))?;
        Ok(())
    }

    /// Makes one request at the account's pace: waits for the turn, the pause
    /// and the window budget, and repeats after the pause when the server
    /// answers 429 (a refusal before processing, so repeating a POST duplicates
    /// nothing). A 403 is a block on the first answer, with no retry.
    fn send(&self, call: Call, path: &str) -> Result<Response, Error> {
        let method = call.method();
        for attempt in 0..ATTEMPTS {
            // The turn lasts until the response is consumed or dropped.
            let mut turn = Turn::take(self.config)?;
            let way = match call {
                Call::SiteGet if bridge::is_open(&self.config.dir) => Way::Tab,
                Call::Get
                | Call::Post(_)
                | Call::Portrait
                | Call::SiteGet
                | Call::StorageGet
                | Call::Download(_) => Way::Direct(self.token()?),
            };
            // Only a 429 leads to another attempt. When that attempt cannot be
            // made, the block is still what happened to the request.
            let blocked = || Error::Blocked {
                service: Service::FamilySearch,
                status: 429,
                path: path.to_owned(),
            };
            // A session not held yet is checked outside the held one's cap; the
            // pause and the window still hold, since they belong to the account.
            if self.unheld.is_none() {
                match turn.capped(self.config) {
                    // The blocked request was the last one the cap allowed.
                    Err(Error::SessionCap { .. }) if attempt > 0 => return Err(blocked()),
                    capped => capped?,
                }
            }
            let wait = turn.wait(self.config, now_ms()?);
            // A pause counts from the millisecond after the answer, so one
            // of exactly the longest wait shows as a millisecond more.
            let longest = Duration::from_secs(self.config.max_wait_s);
            if wait > longest.saturating_add(Duration::from_millis(1)) {
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
            let exchanged = match way {
                Way::Direct(token) => self.direct(call, token, path),
                Way::Tab => bridge::fetch(&self.config.dir, path).map(|answer| Exchange {
                    status: answer.status,
                    processing_ms: number(answer.processing_time.as_deref()),
                    retry_after_s: number(answer.retry_after.as_deref()),
                    location: answer.location,
                    body: answer.body.map_or(Body::Lost, Body::Read),
                }),
            };
            let exchange = match exchanged {
                Ok(exchange) => exchange,
                // The tab went away during the pause after the 429.
                Err(Error::BridgeDetached) if attempt > 0 => return Err(blocked()),
                // No tab took the request: it was not made and does not count.
                Err(error @ Error::BridgeDetached) => return Err(error),
                Err(error) => {
                    // The request may have reached the server, so it still counts.
                    if self.unheld.is_none() {
                        turn.count();
                    }
                    turn.record(self.config, now_ms()?, millis(started.elapsed()), None)?;
                    return Err(error);
                }
            };
            let Exchange {
                status,
                processing_ms,
                retry_after_s,
                location,
                body,
            } = exchange;
            // Without a reported processing time, the time the request took counts.
            let cost_ms = processing_ms.unwrap_or_else(|| millis(started.elapsed()));
            // The server asks for a pause with `Retry-After`, in seconds. After a
            // 429 the pause is long, even when the server asks for little.
            let asked = retry_after_s.map(Duration::from_secs);
            let pause = match (status, call) {
                (429, _) => {
                    let minimum = Duration::from_secs(self.config.pause_429_s);
                    Some(asked.unwrap_or_default().max(minimum))
                }
                // A redirect to the storage is a full image on its way, and
                // full images are what gets an account paused: the next
                // request waits the interval. So does the one after the
                // redirect to a portrait, whose transfer is not counted either.
                (300..=399, Call::Download(_)) | (307, Call::Portrait) => {
                    let interval = Duration::from_millis(self.config.image_interval_ms);
                    Some(asked.unwrap_or_default().max(interval))
                }
                _ => asked,
            };
            // The count and the pause are saved from the headers alone, before any
            // body is read, so a body that stalls or breaks cannot lose them. The
            // turn itself goes on while the body is read. A session being
            // checked spends the account's window, not the held session's cap.
            if self.unheld.is_none() {
                turn.count();
            }
            turn.record(self.config, now_ms()?, cost_ms, pause)?;
            match status {
                200..=299 => {
                    let unread = |action: &str| {
                        anyhow!("não consegui ler a resposta de {method} {path}: {action}")
                    };
                    let text = match body {
                        Body::Read(text) => text,
                        Body::Lost => return Err(unread("a aba não leu o corpo").into()),
                        Body::Unread(mut body) => match call {
                            // A person with no portrait answers 204. Anything
                            // else must not pass for that.
                            Call::Portrait if status != 204 => {
                                return Err(anyhow!(
                                    "o FamilySearch respondeu {status} em {method} {path}"
                                )
                                .into());
                            }
                            // Only what a GET read is used; a POST is told by
                            // its status and `Location`.
                            Call::Post(_) | Call::Portrait => String::new(),
                            Call::Get | Call::SiteGet | Call::StorageGet => {
                                body.read_to_string()
                                    .map_err(|error| unread(&error.to_string()))?
                            }
                            // A thumbnail comes in the answer itself.
                            Call::Download(into) => {
                                File::create(into)
                                    .and_then(|mut file| io::copy(&mut body.as_reader(), &mut file))
                                    .map_err(|error| unread(&error.to_string()))?;
                                String::new()
                            }
                        },
                    };
                    return Ok(Response {
                        body: text,
                        location,
                    });
                }
                // Redirects are not followed: a hidden second request would skip
                // the regulator.
                300..=399 => {
                    // The redirects that are followed, by the caller, once
                    // this request is counted: to a full image and, with a
                    // 307, to a portrait.
                    let followed = match call {
                        Call::Download(_) => true,
                        Call::Portrait => status == 307,
                        Call::Get | Call::Post(_) | Call::SiteGet | Call::StorageGet => false,
                    };
                    if followed && location.is_some() {
                        return Ok(Response {
                            body: String::new(),
                            location,
                        });
                    }
                    let to = location.as_deref().unwrap_or("outro endereço");
                    return match call {
                        // A merged person answers with one.
                        Call::Get | Call::Post(_) | Call::Portrait => Err(Error::Input(format!(
                            "{path} mudou para {to} ({status}); a pessoa pode ter sido \
                             fundida em outra: repita com o id novo"
                        ))),
                        Call::SiteGet => Err(anyhow!(
                            "o site redirecionou {path} para {to} ({status}); se a aba ou a \
                             sessão não está logada, peça o login ao usuário"
                        )
                        .into()),
                        Call::StorageGet | Call::Download(_) => Err(anyhow!(
                            "o armazenamento de imagens redirecionou {path} ({status})"
                        )
                        .into()),
                    };
                }
                401 => {
                    match way {
                        // The session is dead: drop it, so the next command does
                        // not even try. A newer session delivered meanwhile is
                        // left alone.
                        Way::Direct(token) if self.unheld.is_none() => {
                            session::expire(&self.config.dir, token)?;
                        }
                        // A session being checked is not held; the tab's
                        // session is the browser's to renew.
                        Way::Direct(_) | Way::Tab => {}
                    }
                    return Err(Error::SessionExpired);
                }
                403 => {
                    // The image storage refuses one film or one image to the
                    // account by saying so ("Access to artifact denied").
                    // Any other 403 is a block.
                    let about_artifact = matches!(call, Call::StorageGet | Call::Download(_))
                        && match body {
                            Body::Unread(mut body) => body.read_to_string().is_ok_and(|text| {
                                text.to_lowercase().contains("access to artifact denied")
                            }),
                            Body::Read(_) | Body::Lost => false,
                        };
                    return Err(if about_artifact {
                        Error::Restricted {
                            what: path.to_owned(),
                        }
                    } else {
                        Error::Blocked {
                            service: Service::FamilySearch,
                            status,
                            path: path.to_owned(),
                        }
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
            service: Service::FamilySearch,
            status: 429,
            path: path.to_owned(),
        })
    }

    /// The held session, read inside the turn, so a command that waited in
    /// line sees the session that is current when its request would go out.
    /// If that is no longer the session the command started with, it stops: a
    /// check made in one session must not be followed by a write in another.
    fn token(&self) -> Result<&Token, Error> {
        if let Some(token) = self.unheld {
            return Ok(token);
        }
        let current = session::token(&self.config.dir)?;
        match self.token.get() {
            None => Ok(self.token.get_or_init(|| current)),
            Some(first) if *first == current => Ok(first),
            Some(_) => Err(anyhow!("a sessão foi trocada durante o comando").into()),
        }
    }

    /// Makes the request from this process, with the held session.
    fn direct(&self, call: Call, token: &Token, path: &str) -> Result<Exchange, Error> {
        let (base, accept) = match call {
            Call::Get | Call::Post(_) | Call::Portrait => (&self.config.fs_api, MEDIA_TYPE),
            Call::SiteGet => (&self.config.fs_site, SITE_MEDIA_TYPE),
            Call::StorageGet => (&self.config.fs_images, SITE_MEDIA_TYPE),
            Call::Download(_) => (&self.config.fs_images, "image/*"),
        };
        let url = format!("{base}{path}");
        let authorization = format!("Bearer {}", token.as_str());
        let sent = match call {
            Call::Get | Call::Portrait | Call::SiteGet | Call::StorageGet | Call::Download(_) => {
                self.agent
                    .get(&url)
                    .header("Authorization", &authorization)
                    .header("Accept", accept)
                    .call()
            }
            Call::Post(body) => self
                .agent
                .post(&url)
                .header("Authorization", &authorization)
                .header("Accept", accept)
                .content_type(MEDIA_TYPE)
                .send(body.to_string()),
        };
        let (head, body) = sent
            .map_err(|error| {
                anyhow!(error).context(format!("o pedido {} {path} falhou", call.method()))
            })?
            .into_parts();
        let header = |name: &str| head.headers.get(name)?.to_str().ok();
        Ok(Exchange {
            status: head.status.as_u16(),
            processing_ms: number(header("x-processing-time")),
            retry_after_s: number(header("retry-after")),
            location: header("location").map(str::to_owned),
            body: Body::Unread(body),
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
fn number(header: Option<&str>) -> Option<u64> {
    header?.trim().parse().ok()
}
