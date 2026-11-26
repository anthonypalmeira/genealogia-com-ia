use std::{fs, time::Duration};

use crate::{
    env::{TOKEN, TestEnv},
    images::page,
    server::{FakeServer, Reply, Request, person},
    traversal::{ancestry_file, tree},
};

/// A synthetic family of four: A, the parents B and C, and D, a parent of B.
/// A and D have a portrait, B has none, and C was merged into somebody else.
fn portraits(request: &Request) -> Reply {
    let path = request.path.as_str();
    if path.starts_with("/platform/tree/persons?pids=") {
        let persons = ["A", "B", "C", "D"].map(person);
        return tree(request, &persons, &[("B", "A"), ("C", "A"), ("D", "B")]);
    }
    if let Some(id) = path.strip_prefix("/retrato/") {
        // A's portrait is wider than D's, so the two are different files.
        return Reply::bytes(200, "image/jpeg", page(if id == "A" { 60 } else { 50 }, 40));
    }
    let id = path
        .strip_prefix("/platform/tree/persons/")
        .and_then(|rest| rest.strip_suffix("/portrait"))
        .unwrap_or_else(|| panic!("unexpected route: {path}"));
    match id {
        "B" => Reply::empty(204),
        "C" => Reply::empty(301).with_header("Location", "/platform/tree/persons/Z/portrait"),
        _ => Reply::empty(307).with_header(
            "Location",
            &format!("http://{}/retrato/{id}", request.header("Host").unwrap()),
        ),
    }
}

/// The requests for the portrait of a person, in arrival order.
fn asked(fs: &FakeServer, person: &str) -> Vec<Request> {
    let route = format!("/platform/tree/persons/{person}/portrait");
    let about = |request: &Request| request.path == route;
    fs.requests().into_iter().filter(about).collect()
}

#[test]
fn a_missing_portrait_at_the_cdn_is_retried_without_holding_up_the_next_person() {
    let fs = FakeServer::start(|request| {
        if request.path.starts_with("/platform/tree/persons?pids=") {
            tree(request, &["A", "D"].map(person), &[("D", "A")])
        } else if request.path == "/retrato/A" {
            Reply::empty(404)
        } else {
            portraits(request)
        }
    });
    let env = TestEnv::logged_in(&fs);
    let file = ancestry_file(&env, "A");
    let command = ["fs", "retratos", &file];
    let first = env.run(&command);
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(first.stdout, "1\t0\t0\t1\tretratos\n");
    assert!(
        first.stderr.contains("retrato de A pendente de acesso"),
        "{}",
        first.stderr
    );
    assert!(
        first.stderr.contains("404 ao endereço fixo"),
        "{}",
        first.stderr
    );
    assert_eq!(
        fs::read(env.work_dir().join("retratos/D.jpg")).unwrap(),
        page(50, 40)
    );

    let again = env.run(&command);
    assert_eq!(again.code, 0, "{}", again.stderr);
    assert_eq!(again.stdout, "0\t1\t0\t1\tretratos\n");
    assert_eq!(asked(&fs, "A").len(), 2);
    assert_eq!(asked(&fs, "D").len(), 1);
}

#[test]
fn a_cdn_asking_for_a_pause_or_failing_stops_the_batch() {
    for (status, retry_after, wait) in [
        (429, "30", "espere os 30 s"),
        (
            503,
            "Wed, 21 Oct 2026 07:28:00 GMT",
            "espere até Wed, 21 Oct 2026 07:28:00 GMT",
        ),
    ] {
        let fs = FakeServer::start(move |request| {
            if request.path.starts_with("/platform/tree/persons?pids=") {
                tree(request, &["A", "D"].map(person), &[("D", "A")])
            } else if request.path == "/retrato/A" {
                Reply::empty(status).with_header("Retry-After", retry_after)
            } else {
                portraits(request)
            }
        });
        let env = TestEnv::logged_in(&fs);
        let file = ancestry_file(&env, "A");

        let failed = env.run(&["fs", "retratos", &file]);
        assert_eq!(failed.code, 1, "{status}: {}", failed.stderr);
        assert!(failed.stderr.contains(wait), "{status}: {}", failed.stderr);
        assert!(asked(&fs, "D").is_empty(), "{status}");
    }
}

#[test]
fn a_portrait_that_cannot_be_written_stops_the_batch() {
    let fs = FakeServer::start(portraits);
    let env = TestEnv::logged_in(&fs);
    let file = ancestry_file(&env, "A");
    fs::create_dir_all(env.work_dir().join("retratos/A.jpg.part")).unwrap();

    let failed = env.run(&["fs", "retratos", &file]);
    assert_eq!(failed.code, 1, "{}", failed.stderr);
    assert!(
        failed.stderr.contains("não consegui gravar"),
        "{}",
        failed.stderr
    );
    assert!(asked(&fs, "B").is_empty());
}

#[test]
fn a_failed_connection_to_the_portrait_cdn_stops_the_batch() {
    let fs = FakeServer::start(|request| {
        if request.path == "/platform/tree/persons/A/portrait" {
            Reply::empty(307).with_header("Location", "http://127.0.0.1:0/retrato/A")
        } else {
            portraits(request)
        }
    });
    let env = TestEnv::logged_in(&fs);
    let file = ancestry_file(&env, "A");

    let failed = env.run(&["fs", "retratos", &file]);
    assert_eq!(failed.code, 1, "{}", failed.stderr);
    assert!(
        failed.stderr.contains("a transferência da imagem falhou"),
        "{}",
        failed.stderr
    );
    assert!(asked(&fs, "B").is_empty());
}

#[test]
fn the_portraits_of_a_traversal_are_downloaded_once_and_who_has_none_is_not_asked_again() {
    let fs = FakeServer::start(portraits);
    let env = TestEnv::new(&fs).with_var("GARIMPO_INTERVALO_IMAGENS_MS", "300");
    env.deliver_session();
    let file = ancestry_file(&env, "A");
    let folder = env.work_dir().join("retratos");
    let command = ["fs", "retratos", &file, "--pasta", folder.to_str().unwrap()];

    let first = env.run(&command);
    let again = env.run(&command);

    assert_eq!(first.code, 0, "{}", first.stderr);
    // New, already there, with no portrait, pending access, folder.
    assert_eq!(first.stdout, format!("2\t0\t1\t1\t{}\n", folder.display()));
    assert_eq!(fs::read(folder.join("A.jpg")).unwrap(), page(60, 40));
    assert_eq!(fs::read(folder.join("D.jpg")).unwrap(), page(50, 40));
    // The merged person is told apart from the one with no portrait.
    assert!(first.stderr.contains("retrato de C"), "{}", first.stderr);
    // The session goes to FamilySearch only: the address of the image is
    // fetched without it.
    for request in fs.requests() {
        let authorization = request.header("Authorization").map(str::to_owned);
        if request.path.starts_with("/retrato/") {
            assert_eq!(authorization, None, "{}", request.path);
        } else {
            assert_eq!(authorization, Some(format!("Bearer {TOKEN}")));
        }
    }
    // The request after a portrait waits the interval of the images.
    let gap = asked(&fs, "B")[0].arrived - asked(&fs, "A")[0].arrived;
    assert!(gap >= Duration::from_millis(300), "{gap:?}");
    assert_eq!(again.code, 0, "{}", again.stderr);
    assert_eq!(again.stdout, format!("0\t2\t1\t1\t{}\n", folder.display()));
    // Only the merged person, whose access is pending, was asked about again.
    let times = ["A", "B", "C", "D"].map(|person| asked(&fs, person).len());
    assert_eq!(times, [1, 1, 2, 1]);
}
