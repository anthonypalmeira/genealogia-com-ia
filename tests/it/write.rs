use std::{
    fs,
    sync::atomic::{AtomicBool, Ordering},
};

use serde_json::{Value, json};

use crate::{
    env::TestEnv,
    server::{FakeServer, Reply, Request},
};

const SOURCE_URL: &str = "https://exemplo.invalid/acervo/obitos/livro-7/folha-40";
const CREATE_SOURCE: [&str; 13] = [
    "fs",
    "criar-fonte",
    "SINT-001",
    "--titulo",
    "Óbito de Joaquim, 1890",
    "--citacao",
    "Paróquia de Vila Exemplo, Óbitos, livro 7, folha 40.",
    "--url",
    SOURCE_URL,
    "--nota",
    "Faleceu Joaquim, viúvo, aos doze de novembro.",
    "--motivo",
    "O óbito dá a idade e o estado civil.",
];
const ROUND: [&str; 2] = ["--rodada", "2026-11-23 plano aprovado"];

/// The lines of the change log, without the time (which changes on every run).
fn change_log(env: &TestEnv) -> Vec<Value> {
    let Ok(text) = fs::read_to_string(env.work_dir().join("alteracoes.jsonl")) else {
        return Vec::new();
    };
    text.lines()
        .map(|line| {
            let mut change: Value = serde_json::from_str(line).unwrap();
            let when = change.as_object_mut().unwrap().remove("quando").unwrap();
            assert!(when.as_str().unwrap().ends_with('Z'), "{when}");
            change
        })
        .collect()
}

/// The sources of SINT-001 once the new source, SINT-F02, is attached.
fn sources_with_the_new_one() -> Reply {
    Reply::json(
        200,
        json!({
            "persons": [{"id": "SINT-001", "sources": [{"id": "SINT-SR2", "description": "#SINT-F02"}]}],
            "sourceDescriptions": [{
                "id": "SINT-F02",
                "about": SOURCE_URL,
                "titles": [{"value": "Óbito de Joaquim, 1890"}],
            }],
        })
        .to_string(),
    )
}

fn source_created(request: &Request) -> Option<Reply> {
    request
        .is("POST", "/platform/sources/descriptions")
        .then(|| {
            Reply::empty(201).with_header(
                "Location",
                "https://api.familysearch.org/platform/sources/descriptions/SINT-F02",
            )
        })
}

#[test]
fn without_an_authorized_round_the_write_is_refused_before_any_request() {
    let fs = FakeServer::start(|_| Reply::empty(204));
    let env = TestEnv::logged_in(&fs);

    let refused = env.run(&CREATE_SOURCE);

    assert_eq!(refused.code, 6);
    assert!(refused.stderr.contains("--rodada"), "{}", refused.stderr);
    assert_eq!(fs.requests().len(), 0);
    assert_eq!(change_log(&env), Vec::<Value>::new());
}

#[test]
fn create_source_checks_writes_attaches_rereads_and_logs() {
    let attached = AtomicBool::new(false);
    let fs = FakeServer::start(move |request| {
        if let Some(created) = source_created(request) {
            created
        } else if request.is("POST", "/platform/tree/persons/SINT-001") {
            attached.store(true, Ordering::Relaxed);
            Reply::empty(201)
        } else if attached.load(Ordering::Relaxed) {
            sources_with_the_new_one()
        } else {
            Reply::empty(204)
        }
    });
    let env = TestEnv::logged_in(&fs);

    let written = env.run(&[&CREATE_SOURCE[..], &ROUND[..]].concat());

    assert_eq!(written.code, 0, "{}", written.stderr);
    assert_eq!(written.stdout, "anexada\tSINT-F02\tSINT-001\n");
    let requests = fs.requests();
    let routes: Vec<(&str, &str)> = requests.iter().map(Request::route).collect();
    assert_eq!(
        routes,
        [
            ("GET", "/platform/tree/persons/SINT-001/sources"),
            ("POST", "/platform/sources/descriptions"),
            ("POST", "/platform/tree/persons/SINT-001"),
            ("GET", "/platform/tree/persons/SINT-001/sources"),
        ]
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[1].body).unwrap(),
        json!({"sourceDescriptions": [{
            "about": SOURCE_URL,
            "titles": [{"value": "Óbito de Joaquim, 1890"}],
            "citations": [{"value": "Paróquia de Vila Exemplo, Óbitos, livro 7, folha 40."}],
            "notes": [{"text": "Faleceu Joaquim, viúvo, aos doze de novembro."}],
            "attribution": {"changeMessage": "O óbito dá a idade e o estado civil."},
        }]})
    );
    let description = format!("{}/platform/sources/descriptions/SINT-F02", fs.url());
    assert_eq!(
        serde_json::from_str::<Value>(&requests[2].body).unwrap(),
        json!({"persons": [{"sources": [{
            "description": description,
            "attribution": {"changeMessage": "O óbito dá a idade e o estado civil."},
        }]}]})
    );
    assert_eq!(
        requests[2].header("Content-Type"),
        Some("application/x-fs-v1+json")
    );
    let change = |action: &str, verified: bool| {
        json!({
            "acao": action,
            "pessoa": "SINT-001",
            "fonte": "SINT-F02",
            "motivo": "O óbito dá a idade e o estado civil.",
            "rodada": "2026-11-23 plano aprovado",
            "conferida": verified,
        })
    };
    // The attach is logged when the server accepts it and again when the
    // re-read confirms it.
    assert_eq!(
        change_log(&env),
        [
            change("criar-fonte", true),
            change("anexar-fonte", false),
            change("anexar-fonte", true),
        ]
    );
}

#[test]
fn a_source_the_person_already_has_is_not_duplicated() {
    let fs = FakeServer::start(|_| sources_with_the_new_one());
    let env = TestEnv::logged_in(&fs);

    let repeated = env.run(&[&CREATE_SOURCE[..], &ROUND[..]].concat());
    let attach_again = env.run(
        &[
            &[
                "fs",
                "anexar-fonte",
                "SINT-001",
                "--fonte",
                "SINT-F02",
                "--motivo",
                "Repetição.",
            ],
            &ROUND[..],
        ]
        .concat(),
    );

    assert_eq!(repeated.code, 0, "{}", repeated.stderr);
    assert_eq!(repeated.stdout, "ja-existe\tSINT-F02\tSINT-001\n");
    assert_eq!(attach_again.code, 0, "{}", attach_again.stderr);
    assert_eq!(attach_again.stdout, "ja-existe\tSINT-F02\tSINT-001\n");
    let methods: Vec<String> = fs.requests().into_iter().map(|r| r.method).collect();
    assert_eq!(methods, ["GET", "GET"]);
    assert_eq!(change_log(&env), Vec::<Value>::new());
}

#[test]
fn a_failed_attach_logs_no_success_and_says_how_to_finish() {
    let fs = FakeServer::start(|request| {
        if let Some(created) = source_created(request) {
            created
        } else if request.method == "POST" {
            Reply::empty(500)
        } else {
            Reply::empty(204)
        }
    });
    let env = TestEnv::logged_in(&fs);

    let failed = env.run(&[&CREATE_SOURCE[..], &ROUND[..]].concat());

    assert_eq!(failed.code, 1);
    assert_eq!(failed.stdout, "");
    assert!(
        failed
            .stderr
            .contains("garimpo fs anexar-fonte SINT-001 --fonte SINT-F02"),
        "{}",
        failed.stderr
    );
    // Only the creation of the source, which did happen, is logged.
    let actions: Vec<Value> = change_log(&env)
        .into_iter()
        .map(|change| change["acao"].clone())
        .collect();
    assert_eq!(actions, [json!("criar-fonte")]);
}

#[test]
fn an_attach_that_does_not_show_on_reread_is_logged_as_not_confirmed() {
    // The server accepts the request, but the source is not among the person's.
    let fs = FakeServer::start(|request| {
        if request.method == "POST" {
            Reply::empty(201)
        } else {
            Reply::empty(204)
        }
    });
    let env = TestEnv::logged_in(&fs);

    let failed = env.run(
        &[
            &[
                "fs",
                "anexar-fonte",
                "SINT-001",
                "--fonte",
                "SINT-F02",
                "--motivo",
                "Óbito.",
            ],
            &ROUND[..],
        ]
        .concat(),
    );

    assert_eq!(failed.code, 1);
    assert!(failed.stderr.contains("releitura"), "{}", failed.stderr);
    // The accepted request is on the log, marked as not confirmed.
    assert_eq!(
        change_log(&env),
        [json!({
            "acao": "anexar-fonte",
            "pessoa": "SINT-001",
            "fonte": "SINT-F02",
            "motivo": "Óbito.",
            "rodada": "2026-11-23 plano aprovado",
            "conferida": false,
        })]
    );
}

#[test]
fn create_note_checks_writes_rereads_and_logs_without_duplicating() {
    let created = AtomicBool::new(false);
    let fs = FakeServer::start(move |request| {
        if request.is("POST", "/platform/tree/persons/SINT-001/notes") {
            created.store(true, Ordering::Relaxed);
            Reply::empty(201).with_header(
                "Location",
                "https://api.familysearch.org/platform/tree/persons/SINT-001/notes/SINT-N1",
            )
        } else if created.load(Ordering::Relaxed) {
            Reply::recorded("notes.json")
        } else {
            Reply::empty(204)
        }
    });
    let env = TestEnv::logged_in(&fs);
    let command = [
        &[
            "fs",
            "criar-nota",
            "SINT-001",
            "--assunto",
            "Hipótese sobre os avós",
            "--texto",
            "Os avós paternos podem estar no inventário de 1850.\nPróximo passo: ler o título de herdeiros.",
            "--motivo",
            "Guardar a hipótese e o próximo passo.",
        ],
        &ROUND[..],
    ]
    .concat();

    let note = env.run(&command);
    let repeated = env.run(&command);

    assert_eq!(note.code, 0, "{}", note.stderr);
    assert_eq!(note.stdout, "criada\tSINT-N1\tSINT-001\n");
    assert_eq!(repeated.code, 0, "{}", repeated.stderr);
    assert_eq!(repeated.stdout, "ja-existe\t-\tSINT-001\n");
    let requests = fs.requests();
    let posts: Vec<&Request> = requests
        .iter()
        .filter(|request| request.method == "POST")
        .collect();
    assert_eq!(posts.len(), 1);
    assert_eq!(
        serde_json::from_str::<Value>(&posts[0].body).unwrap(),
        json!({"persons": [{"notes": [{
            "subject": "Hipótese sobre os avós",
            "text": "Os avós paternos podem estar no inventário de 1850.\nPróximo passo: ler o título de herdeiros.",
        }]}]})
    );
    let change = |verified: bool| {
        json!({
            "acao": "criar-nota",
            "pessoa": "SINT-001",
            "nota": "SINT-N1",
            "motivo": "Guardar a hipótese e o próximo passo.",
            "rodada": "2026-11-23 plano aprovado",
            "conferida": verified,
        })
    };
    assert_eq!(change_log(&env), [change(false), change(true)]);
}
