use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use serde_json::{Value, json};
use ureq::Agent;

use crate::{
    env::{TOKEN, TestEnv, output},
    server::{FakeServer, Reply, full_text, page_image},
};

/// What the fake tab says it is, so a test can tell which way a request went.
const TAB_AGENT: &str = "aba-sintetica";

/// Plays the loop the logged tab runs: asks the bridge for work, makes each
/// request to the fake FamilySearch as the page would, and posts the answer.
/// Asked for the page's session, it gives `cookie`, as the page's
/// `fssessionid`.
struct FakeTab {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl FakeTab {
    fn attach(bridge: &str, site: &str, cookie: Option<&str>) -> Self {
        let (bridge, site) = (bridge.to_owned(), site.to_owned());
        let cookie = cookie.map(str::to_owned);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let http: Agent = Agent::config_builder()
            .http_status_as_error(false)
            // Short, so the tab notices soon that the test is over.
            .timeout_global(Some(Duration::from_secs(1)))
            .build()
            .into();
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                // No work within the timeout: ask again, as after a 204.
                let Ok(mut next) = http.get(format!("{bridge}/proxima")).call() else {
                    continue;
                };
                match next.status().as_u16() {
                    200 => {}
                    204 => continue,
                    _ => break,
                }
                let job: Value = next.body_mut().read_json().unwrap();
                let accepted = http
                    .post(format!("{bridge}/aceito"))
                    .send(job["id"].to_string())
                    .unwrap();
                if accepted.status() != 204 {
                    continue;
                }
                if job["session"] == true {
                    let status = if cookie.is_some() { 200 } else { 401 };
                    let answer = json!({"status": status, "body": cookie});
                    http.post(format!("{bridge}/resposta"))
                        .send(json!({"id": job["id"], "answer": answer}).to_string())
                        .unwrap();
                    continue;
                }
                let mut response = http
                    .get(format!("{site}{}", job["path"].as_str().unwrap()))
                    .header("Accept", "application/json")
                    .header("User-Agent", TAB_AGENT)
                    .call()
                    .unwrap();
                let header = |name: &str| {
                    let value = response.headers().get(name)?;
                    Some(value.to_str().unwrap().to_owned())
                };
                let answer = json!({
                    "status": response.status().as_u16(),
                    "retryAfter": header("retry-after"),
                    "processingTime": header("x-processing-time"),
                    "location": header("location"),
                    "body": response.body_mut().read_to_string().unwrap(),
                });
                http.post(format!("{bridge}/resposta"))
                    .send(json!({"id": job["id"], "answer": answer}).to_string())
                    .unwrap();
            }
        });
        Self {
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for FakeTab {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // A tab that panicked has already failed the test.
        let _ = self.worker.take().unwrap().join();
    }
}

fn one_page_image() -> FakeServer {
    FakeServer::start(|_| full_text(1, &[page_image("SINT-PAG-001", "auto de partilha")]))
}

#[test]
fn the_bridge_and_the_direct_way_give_the_same_result() {
    let fs = one_page_image();
    let search = ["busca", "texto", "+partilha", "--json"];
    // With the bridge, no session is delivered: the tab has its own.
    let bridged = TestEnv::new(&fs);
    let direct = TestEnv::logged_in(&fs);

    let opened = bridged.run(&["ponte", "abrir", "--json"]);
    assert_eq!(opened.code, 0, "{}", opened.stderr);
    let opened: Value = serde_json::from_str(&opened.stdout).unwrap();
    let address = opened["endereco"].as_str().unwrap();
    // The snippet for the tab carries the address of the bridge and the site.
    let snippet = opened["trecho"].as_str().unwrap();
    assert!(snippet.contains(&format!("\"{address}\"")), "{snippet}");
    assert!(snippet.contains(&format!("\"{}\"", fs.url())), "{snippet}");
    let tab = FakeTab::attach(address, fs.url(), None);
    let through_the_tab = bridged.run(&search);
    let attached = bridged.run(&["ponte", "estado"]);
    let closed = bridged.run(&["ponte", "fechar"]);
    drop(tab);
    let directly = direct.run(&search);

    assert_eq!(through_the_tab.code, 0, "{}", through_the_tab.stderr);
    assert_eq!(directly.code, 0, "{}", directly.stderr);
    assert_eq!(through_the_tab.stdout, directly.stdout);
    assert!(
        directly.stdout.contains("ark:/61903/3:1:SINT-PAG-001"),
        "{}",
        directly.stdout
    );
    assert_eq!(bridged.coverage()[0]["resultado"], "lido");
    assert_eq!(attached.stdout, "ponte ligada\n", "{}", attached.stderr);
    assert_eq!(closed.stdout, "ponte fechada\n", "{}", closed.stderr);
    // The same request, by the two ways: first from the tab, then from garimpo.
    let requests = fs.requests();
    let routes: Vec<&str> = requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0], routes[1]);
    let agents: Vec<&str> = requests
        .iter()
        .map(|request| request.header("User-Agent").unwrap())
        .collect();
    assert_eq!(agents[0], TAB_AGENT);
    assert!(agents[1].starts_with("garimpo/"), "{}", agents[1]);
    // Through the tab, garimpo itself sends nothing: not even the session.
    assert_eq!(requests[0].header("Authorization"), None);
}

#[test]
fn a_tab_that_takes_too_long_to_accept_is_refused_and_the_command_exits_7() {
    let fs = one_page_image();
    let env = TestEnv::logged_in(&fs);
    // With no bridge there is no tab either.
    assert_eq!(env.run(&["ponte", "estado"]).code, 7);
    let opened = env.run(&["ponte", "abrir", "--json"]);
    assert_eq!(opened.code, 0, "{}", opened.stderr);
    let opened: Value = serde_json::from_str(&opened.stdout).unwrap();
    let address = opened["endereco"].as_str().unwrap();
    let http: Agent = Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();

    let search = env
        .command(&["busca", "texto", "+partilha"])
        .spawn()
        .unwrap();
    // The tab gets the request and only accepts it after the command gave up.
    let job: Value = http
        .get(format!("{address}/proxima"))
        .call()
        .unwrap()
        .body_mut()
        .read_json()
        .unwrap();
    let search = output(search);
    let accepted = http
        .post(format!("{address}/aceito"))
        .send(job["id"].to_string())
        .unwrap();

    assert_eq!(search.code, 7, "{}", search.stderr);
    assert!(
        search.stderr.contains("avalie o trecho"),
        "{}",
        search.stderr
    );
    // The tab is told not to make a request nobody would count.
    assert_eq!(accepted.status(), 409);
    assert_eq!(fs.requests().len(), 0);
    assert_eq!(env.coverage(), Vec::<Value>::new());
}

#[test]
fn a_429_followed_by_the_tab_going_away_is_still_recorded_as_a_block() {
    let fs = one_page_image();
    let env = TestEnv::new(&fs).with_var("GARIMPO_PAUSA_429_S", "0");
    let opened = env.run(&["ponte", "abrir", "--json"]);
    assert_eq!(opened.code, 0, "{}", opened.stderr);
    let opened: Value = serde_json::from_str(&opened.stdout).unwrap();
    let address = opened["endereco"].as_str().unwrap();
    let http: Agent = Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();

    let search = env
        .command(&["busca", "texto", "+partilha"])
        .spawn()
        .unwrap();
    // The tab answers the request with the site's 429 and is never seen again.
    let job: Value = http
        .get(format!("{address}/proxima"))
        .call()
        .unwrap()
        .body_mut()
        .read_json()
        .unwrap();
    http.post(format!("{address}/aceito"))
        .send(job["id"].to_string())
        .unwrap();
    let answer = json!({"status": 429, "retryAfter": "0", "body": null});
    http.post(format!("{address}/resposta"))
        .send(json!({"id": job["id"], "answer": answer}).to_string())
        .unwrap();
    let search = output(search);

    // Not exit 7: the second attempt had no tab, but the request got a 429.
    assert_eq!(search.code, 4, "{}", search.stderr);
    assert_eq!(env.coverage()[0]["resultado"], "pendente-de-acesso");
}

/// Opens the bridge for `env` and attaches a fake tab whose page has `cookie`.
fn tab_with(env: &TestEnv, fs: &FakeServer, cookie: Option<&str>) -> FakeTab {
    let opened = env.run(&["ponte", "abrir", "--json"]);
    assert_eq!(opened.code, 0, "{}", opened.stderr);
    let opened: Value = serde_json::from_str(&opened.stdout).unwrap();
    FakeTab::attach(opened["endereco"].as_str().unwrap(), fs.url(), cookie)
}

/// A fake FamilySearch that knows only the sessions in `valid`, and answers
/// `sessao-vazia` with no user at all.
fn knowing(valid: &'static [&'static str]) -> FakeServer {
    FakeServer::start(move |request| match request.header("Authorization") {
        Some("Bearer sessao-vazia") => Reply::empty(204),
        Some(header)
            if valid
                .iter()
                .any(|token| header == format!("Bearer {token}")) =>
        {
            Reply::recorded("current-user.json")
        }
        _ => Reply::empty(401),
    })
}

#[test]
fn the_tab_hands_over_its_session_checked_and_never_shown() {
    let fs = knowing(&[TOKEN, "sessao-da-aba"]);
    // The held session already made every request its cap allows: the new
    // one is checked outside that cap.
    let env = TestEnv::logged_in(&fs).with_var("GARIMPO_TETO_SESSAO", "1");
    assert_eq!(env.run(&["sessao", "conferir"]).code, 0);
    let tab = tab_with(&env, &fs, Some("sessao-da-aba"));

    let delivered = env.run(&["sessao", "entregar", "--pela-ponte"]);
    drop(tab);
    let checked = env.run(&["sessao", "conferir"]);

    assert_eq!(delivered.code, 0, "{}", delivered.stderr);
    assert_eq!(delivered.stdout, "sessao guardada\n");
    assert_eq!(checked.code, 0, "{}", checked.stderr);
    for shown in [
        &delivered.stdout,
        &delivered.stderr,
        &checked.stdout,
        &checked.stderr,
    ] {
        assert!(!shown.contains("sessao-da-aba"), "{shown}");
    }
    for dir in [env.state_dir(), env.work_dir()] {
        for file in walk(dir) {
            let bytes = std::fs::read(&file).unwrap_or_default();
            assert!(
                !String::from_utf8_lossy(&bytes).contains("sessao-da-aba"),
                "{}",
                file.display()
            );
        }
    }
}

#[test]
fn a_stale_or_missing_session_in_the_tab_keeps_the_held_one() {
    for (cookie, code) in [
        (Some("sessao-velha"), 3),
        (Some("sessao-vazia"), 1),
        (None, 3),
    ] {
        let fs = knowing(&[TOKEN]);
        // A refused check spends nothing of the held session's cap.
        let env = TestEnv::logged_in(&fs).with_var("GARIMPO_TETO_SESSAO", "1");
        let tab = tab_with(&env, &fs, cookie);

        let delivered = env.run(&["sessao", "entregar", "--pela-ponte"]);
        drop(tab);
        let checked = env.run(&["sessao", "conferir"]);

        assert_eq!(delivered.code, code, "{cookie:?}: {}", delivered.stderr);
        assert_eq!(checked.code, 0, "{cookie:?}: {}", checked.stderr);
    }
}

/// Every file under `dir`.
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else if path.is_file() {
            files.push(path);
        }
    }
    files
}
