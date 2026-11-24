use serde_json::json;

use crate::{
    env::TestEnv,
    server::{FakeServer, Reply, full_text, page_image},
};

#[test]
fn a_403_is_recorded_as_pending_access_never_as_a_negative() {
    let fs = FakeServer::start(|_| Reply::empty(403));
    let env = TestEnv::logged_in(&fs);

    let blocked = env.run(&["busca", "texto", "+inventario", "--lugar", "Vila Exemplo"]);
    let asked = env.run(&["cobertura", "consultar", "inventario"]);

    assert_eq!(blocked.code, 4, "{}", blocked.stderr);
    assert_eq!(blocked.stdout, "");
    // The next step points to the bridge: the site refuses outside a browser.
    assert!(
        blocked.stderr.contains("garimpo ponte abrir"),
        "{}",
        blocked.stderr
    );
    assert_eq!(fs.requests().len(), 1);
    let coverage = env.coverage();
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0]["resultado"], "pendente-de-acesso");
    assert_eq!(coverage[0]["total"], json!(null));
    assert_eq!(coverage[0]["vistos"], 0);
    assert_eq!(
        coverage[0]["motivo"],
        "bloqueio: o FamilySearch respondeu 403 em \
         /service/search/fulltext/search?q.text=%2Binventario&q.anyPlace=Vila+Exemplo\
         &count=20&offset=0"
    );
    assert_eq!(asked.code, 0, "{}", asked.stderr);
    assert!(
        asked.stdout.starts_with("pendente-de-acesso\t"),
        "{}",
        asked.stdout
    );
}

#[test]
fn a_429_that_asks_for_a_long_pause_is_recorded_as_pending_access_too() {
    let fs = FakeServer::start(|_| Reply::empty(429).with_header("Retry-After", "1"));
    // After a 429 the pause is long (60 s), more than this command accepts to wait.
    let env = TestEnv::new(&fs).with_var("GARIMPO_ESPERA_MAX_S", "5");
    env.deliver_session();

    let paused = env.run(&["busca", "registros", "--sobrenome", "Silva"]);

    assert_eq!(paused.code, 4, "{}", paused.stderr);
    assert_eq!(fs.requests().len(), 1);
    let coverage = env.coverage();
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0]["busca"], "registros");
    assert_eq!(coverage[0]["consulta"], "sobrenome:Silva");
    assert_eq!(coverage[0]["resultado"], "pendente-de-acesso");
}

#[test]
fn a_429_on_the_last_request_of_the_session_cap_is_still_a_block() {
    let fs = FakeServer::start(|_| Reply::empty(429).with_header("Retry-After", "0"));
    let env = TestEnv::new(&fs)
        .with_var("GARIMPO_TETO_SESSAO", "1")
        .with_var("GARIMPO_PAUSA_429_S", "0");
    env.deliver_session();

    let blocked = env.run(&["busca", "texto", "+inventario"]);

    // The cap leaves no room for another attempt, but what happened was a 429.
    assert_eq!(blocked.code, 4, "{}", blocked.stderr);
    assert_eq!(fs.requests().len(), 1);
    assert_eq!(env.coverage()[0]["resultado"], "pendente-de-acesso");
}

#[test]
fn a_search_the_server_fails_to_answer_is_recorded_with_the_reason() {
    let fs = FakeServer::start(|_| Reply::empty(500));
    let env = TestEnv::logged_in(&fs);

    let failed = env.run(&["busca", "texto", "+inventario"]);

    assert_eq!(failed.code, 1, "{}", failed.stderr);
    let coverage = env.coverage();
    assert_eq!(coverage.len(), 1);
    // Not a negative: the search is still to be made.
    assert_eq!(coverage[0]["resultado"], "pendente-de-acesso");
    assert_eq!(
        coverage[0]["motivo"],
        "o FamilySearch respondeu 500 em GET \
         /service/search/fulltext/search?q.text=%2Binventario&count=20&offset=0"
    );
}

#[test]
fn a_search_refused_for_login_or_for_a_missing_route_is_pending_access_too() {
    let fs = FakeServer::start(|request| {
        if request.path.contains("/fulltext/") {
            Reply::empty(404)
        } else {
            Reply::empty(401)
        }
    });
    let env = TestEnv::logged_in(&fs);

    let missing = env.run(&["busca", "texto", "+inventario"]);
    let expired = env.run(&["busca", "registros", "--sobrenome", "Silva"]);

    assert_eq!(missing.code, 2, "{}", missing.stderr);
    assert_eq!(expired.code, 3, "{}", expired.stderr);
    let coverage = env.coverage();
    let recorded: Vec<(&str, &str)> = coverage
        .iter()
        .map(|record| {
            (
                record["resultado"].as_str().unwrap(),
                record["motivo"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        recorded,
        [
            (
                "pendente-de-acesso",
                "entrada inválida: o FamilySearch não achou \
                 /service/search/fulltext/search?q.text=%2Binventario&count=20&offset=0 (404)"
            ),
            (
                "pendente-de-acesso",
                "login necessário: a sessão do FamilySearch expirou"
            ),
        ]
    );
}

#[test]
fn every_search_is_recorded_and_the_coverage_tells_what_was_already_searched() {
    let fs = FakeServer::start(|request| {
        let one = [page_image("SINT-PAG-001", "auto de partilha")];
        match request.param("q.text").unwrap().as_str() {
            "+partilha" => full_text(1, &one),
            // One page image out of forty: the search stays open.
            "+inventario" => full_text(40, &one),
            _ => full_text(0, &[]),
        }
    });
    let env = TestEnv::logged_in(&fs);
    let place = ["--lugar", "Vila Exemplo", "--de", "1840", "--ate", "1860"];

    for text in ["+partilha", "+inventario", "+testamento"] {
        let searched = env.run(&[&["busca", "texto", text], &place[..]].concat());
        assert_eq!(searched.code, 0, "{}", searched.stderr);
    }
    let all = env.run(&["cobertura", "consultar"]);
    // Accents and case do not matter, and every term has to be there.
    let negative = env.run(&["cobertura", "consultar", "VILA EXÉMPLO", "testamento"]);
    let never = env.run(&["cobertura", "consultar", "habilitacao"]);

    let coverage = env.coverage();
    assert_eq!(coverage.len(), 3);
    let mut first = coverage[0].clone();
    let when = first.as_object_mut().unwrap().remove("quando").unwrap();
    assert_eq!(when.as_str().unwrap().len(), "2026-01-01T00:00:00Z".len());
    assert_eq!(
        first,
        json!({
            "busca": "texto",
            "consulta": "+partilha",
            "lugar": "Vila Exemplo",
            "de": 1840,
            "ate": 1860,
            "inicio": 0,
            "total": 1,
            "vistos": 1,
            "itens": ["ark:/61903/3:1:SINT-PAG-001"],
            "resultado": "lido",
        })
    );
    assert_eq!(all.code, 0, "{}", all.stderr);
    // Without the moment of each search, which varies.
    let lines: Vec<String> = all
        .stdout
        .lines()
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            [&fields[..1], &fields[2..]].concat().join("\t")
        })
        .collect();
    assert_eq!(
        lines,
        [
            "lido\ttexto\t+partilha\tVila Exemplo\t1840-1860\t1\t1",
            "parcial\ttexto\t+inventario\tVila Exemplo\t1840-1860\t1\t40",
            "negativo\ttexto\t+testamento\tVila Exemplo\t1840-1860\t0\t0",
        ]
    );
    assert_eq!(negative.stdout.lines().count(), 1, "{}", negative.stdout);
    assert!(
        negative.stdout.starts_with("negativo\t"),
        "{}",
        negative.stdout
    );
    assert_eq!((never.code, never.stdout.as_str()), (0, ""));
    assert!(
        never.stderr.contains("nenhuma busca registrada"),
        "{}",
        never.stderr
    );
}
