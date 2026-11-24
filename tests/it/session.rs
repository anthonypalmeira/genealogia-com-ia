use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use crate::{
    env::{TOKEN, TestEnv},
    server::{FakeServer, Reply},
};

#[test]
fn session_delivered_once_serves_the_following_commands() {
    let fs = FakeServer::start(|_| Reply::recorded("current-user.json"));
    let env = TestEnv::new(&fs);

    env.deliver_session();
    let checked = env.run(&["sessao", "conferir"]);

    assert_eq!(checked.code, 0, "{}", checked.stderr);
    assert_eq!(checked.stdout, "sessao valida\n");
    let requests = fs.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].route(), ("GET", "/platform/users/current"));
    assert_eq!(
        requests[0].header("Authorization"),
        Some(format!("Bearer {TOKEN}").as_str())
    );
}

#[test]
fn without_a_session_the_command_asks_for_login_and_makes_no_request() {
    let fs = FakeServer::start(|_| Reply::recorded("current-user.json"));
    let env = TestEnv::new(&fs);

    let checked = env.run(&["sessao", "conferir"]);

    assert_eq!(checked.code, 3);
    assert!(
        checked.stderr.contains("login necessário"),
        "{}",
        checked.stderr
    );
    assert!(
        checked
            .stderr
            .contains("peça ao usuário para entrar no FamilySearch"),
        "{}",
        checked.stderr
    );
    assert_eq!(fs.requests().len(), 0);
}

#[test]
fn expired_session_asks_for_login_and_is_dropped() {
    let fs = FakeServer::start(|_| Reply::empty(401));
    let env = TestEnv::logged_in(&fs);

    let expired = env.run(&["sessao", "conferir"]);
    let after = env.run(&["sessao", "conferir"]);

    assert_eq!(expired.code, 3);
    assert!(
        expired.stderr.contains("login necessário"),
        "{}",
        expired.stderr
    );
    assert!(
        expired
            .stderr
            .contains("peça ao usuário para entrar no FamilySearch"),
        "{}",
        expired.stderr
    );
    // The dead session was dropped: the second command does not even call the server.
    assert_eq!(after.code, 3);
    assert_eq!(fs.requests().len(), 1);
}

#[test]
fn session_token_never_goes_to_disk() {
    let fs = FakeServer::start(|_| Reply::recorded("current-user.json"));
    let env = TestEnv::logged_in(&fs);
    assert_eq!(env.run(&["sessao", "conferir"]).code, 0);

    for dir in [env.state_dir(), env.work_dir()] {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            // The session socket is not a regular file: it stores no data.
            if path.is_file() {
                let content = std::fs::read(&path).unwrap();
                assert!(
                    !String::from_utf8_lossy(&content).contains(TOKEN),
                    "token written to {}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn oauth_with_pkce_exchanges_the_code_for_the_token_and_holds_the_session() {
    const OAUTH_TOKEN: &str = "token-sintetico-do-oauth";
    let fs = FakeServer::start(|request| {
        if request.is("POST", "/cis-web/oauth2/v3/token") {
            Reply::json(
                200,
                format!(r#"{{"access_token":"{OAUTH_TOKEN}","token_type":"Bearer"}}"#),
            )
        } else {
            Reply::recorded("current-user.json")
        }
    });
    let env = TestEnv::new(&fs);

    // The command prints the authorization URL and waits for the browser to come back.
    let mut login = env
        .command(&[
            "sessao",
            "oauth",
            "--chave",
            "chave-sintetica",
            "--porta",
            "0",
        ])
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(login.stdout.take().unwrap());
    let mut authorization = String::new();
    stdout.read_line(&mut authorization).unwrap();
    let (endpoint, query) = authorization.trim_end().split_once('?').unwrap();
    let sent: HashMap<String, String> = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();

    assert_eq!(
        endpoint,
        format!("{}/cis-web/oauth2/v3/authorization", fs.url())
    );
    assert_eq!(sent["response_type"], "code");
    assert_eq!(sent["client_id"], "chave-sintetica");
    assert_eq!(sent["code_challenge_method"], "S256");
    let redirect = &sent["redirect_uri"];
    assert!(redirect.starts_with("http://127.0.0.1:"), "{redirect}");

    // The test plays the browser: FamilySearch redirects to the loopback address.
    let page = ureq::get(format!(
        "{redirect}/?code=codigo-sintetico&state={}",
        sent["state"]
    ))
    .call()
    .unwrap();
    assert_eq!(page.status(), 200);
    let mut rest = String::new();
    stdout.read_to_string(&mut rest).unwrap();
    assert_eq!(login.wait().unwrap().code(), Some(0));
    assert_eq!(rest, "sessao guardada\n");

    // The code exchange sent the verifier that matches the challenge (S256).
    let requests = fs.requests();
    let exchange: HashMap<String, String> = form_urlencoded::parse(requests[0].body.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(exchange["grant_type"], "authorization_code");
    assert_eq!(exchange["code"], "codigo-sintetico");
    assert_eq!(exchange["client_id"], "chave-sintetica");
    assert_eq!(&exchange["redirect_uri"], redirect);
    let verifier = &exchange["code_verifier"];
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    assert_eq!(challenge, sent["code_challenge"]);
    assert!(
        verifier.len() >= 43,
        "a PKCE verifier has at least 43 characters, got {}",
        verifier.len()
    );

    // The OAuth session serves the following commands.
    assert_eq!(env.run(&["sessao", "conferir"]).code, 0);
    assert_eq!(
        fs.requests()[1].header("Authorization"),
        Some(format!("Bearer {OAUTH_TOKEN}").as_str())
    );
}
