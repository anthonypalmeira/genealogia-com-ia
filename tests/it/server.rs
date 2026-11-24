//! A fake FamilySearch: a local HTTP server that answers with synthetic
//! recordings and keeps every request it received, so a test can check what
//! the binary sent.

use std::{
    fs,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use tiny_http::{Header, Response, Server};

#[derive(Clone, Debug)]
pub(crate) struct Request {
    pub(crate) method: String,
    /// The path with its query, as it arrived.
    pub(crate) path: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: String,
    pub(crate) arrived: Instant,
}

impl Request {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub(crate) fn is(&self, method: &str, path: &str) -> bool {
        self.route() == (method, path)
    }

    /// One parameter of the query, decoded.
    pub(crate) fn param(&self, name: &str) -> Option<String> {
        let (_, query) = self.path.split_once('?')?;
        form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    }

    /// The method and the path, to compare with `assert_eq!`.
    pub(crate) fn route(&self) -> (&str, &str) {
        (&self.method, &self.path)
    }
}

pub(crate) struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Reply {
    pub(crate) fn empty(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: String::new(),
        }
    }

    pub(crate) fn json(status: u16, body: String) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "application/x-fs-v1+json".into())],
            body,
        }
    }

    /// A recorded response from `tests/fixtures`.
    pub(crate) fn recorded(name: &str) -> Self {
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        Self::json(200, fs::read_to_string(file).unwrap())
    }

    pub(crate) fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

pub(crate) struct FakeServer {
    url: String,
    requests: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl FakeServer {
    /// Starts the server on a free port. Several workers answer at once, so
    /// simultaneous requests show up as simultaneous.
    pub(crate) fn start(respond: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> Self {
        let http = Arc::new(Server::http("127.0.0.1:0").unwrap());
        let url = format!("http://{}", http.server_addr());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let respond = Arc::new(respond);
        let workers = (0..4)
            .map(|_| {
                let (http, requests, stop, respond) = (
                    Arc::clone(&http),
                    Arc::clone(&requests),
                    Arc::clone(&stop),
                    Arc::clone(&respond),
                );
                thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        let Some(mut raw) = http.recv_timeout(Duration::from_millis(50)).unwrap()
                        else {
                            continue;
                        };
                        let mut body = String::new();
                        raw.as_reader().read_to_string(&mut body).unwrap();
                        let request = Request {
                            method: raw.method().to_string(),
                            path: raw.url().to_owned(),
                            headers: raw
                                .headers()
                                .iter()
                                .map(|h| (h.field.to_string(), h.value.to_string()))
                                .collect(),
                            body,
                            arrived: Instant::now(),
                        };
                        requests.lock().unwrap().push(request.clone());
                        let reply = respond(&request);
                        let mut response =
                            Response::from_string(reply.body).with_status_code(reply.status);
                        for (name, value) in reply.headers {
                            response.add_header(
                                Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap(),
                            );
                        }
                        // The binary may already be gone (a test that stops it early).
                        let _ = raw.respond(response);
                    }
                })
            })
            .collect();
        Self {
            url,
            requests,
            stop,
            workers,
        }
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    /// The requests received so far, in arrival order.
    pub(crate) fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for worker in self.workers.drain(..) {
            // A worker that panicked has already failed the test.
            let _ = worker.join();
        }
    }
}

/// The ids asked of the batch persons route.
pub(crate) fn pids(request: &Request) -> Vec<String> {
    let list = request
        .path
        .strip_prefix("/platform/tree/persons?pids=")
        .unwrap_or_else(|| panic!("unexpected route: {}", request.path));
    list.split(',').map(str::to_owned).collect()
}

/// A synthetic person, long dead, as the tree routes send it.
pub(crate) fn person(id: &str) -> Value {
    json!({
        "id": id,
        "living": false,
        "names": [{"nameForms": [{"fullText": format!("Pessoa Sintética {id}")}]}],
    })
}

/// A reference to a person, as relationships carry it.
pub(crate) fn reference(id: &str) -> Value {
    json!({
        "resource": format!("https://api.familysearch.org/platform/tree/persons/{id}"),
        "resourceId": id,
    })
}

/// A synthetic page image of the full text, as the internal route sends it.
pub(crate) fn page_image(id: &str, text: &str) -> Value {
    json!({
        "id": format!("3:1:{id}"),
        "sourceUrl": format!("https://www.familysearch.org/ark:/61903/3:1:{id}"),
        "collectionId": "9990001",
        "collectionTitle": "Coleção Sintética de Inventários",
        "content": {
            "recordDate": "1850",
            "recordType": "Inventário",
            "recordPlace": "Vila Exemplo, Minas Gerais, Brasil",
            "title": "Vila Exemplo, Inventário 1850",
            "textDocument": text,
            "entities": [],
            "highlightTexts": [],
        },
    })
}

/// An answer of the full-text route: `total` page images in all, of which
/// `entries` came.
pub(crate) fn full_text(total: usize, entries: &[Value]) -> Reply {
    Reply::json(
        200,
        json!({"results": total, "index": 0, "entries": entries, "facets": []}).to_string(),
    )
}
