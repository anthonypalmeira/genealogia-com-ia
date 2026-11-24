use std::fs;

use serde_json::Value;

use crate::{
    env::{TestEnv, output},
    server::{FakeServer, full_text, page_image},
};

/// A group of 250 page images. Past the total the route does not stop: it
/// goes around and repeats the first ones, as the real one does.
fn group_of_250() -> FakeServer {
    FakeServer::start(|request| {
        let offset: usize = request.param("offset").unwrap().parse().unwrap();
        let entries: Vec<Value> = (offset..offset + 100)
            .map(|n| page_image(&format!("SINT-{:03}", n % 250), "auto de partilha"))
            .collect();
        full_text(250, &entries)
    })
}

/// The lines of a pulled group's file, after its header.
fn pulled_ids(file: &std::path::Path) -> Vec<String> {
    let content = fs::read_to_string(file).unwrap();
    content
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["id"].to_string())
        .collect()
}

#[test]
fn pulling_a_group_stops_at_the_total_when_the_route_repeats_pages() {
    let fs = group_of_250();
    let env = TestEnv::logged_in(&fs);

    let pulled = env.run(&["busca", "grupo", "009990001", "--saida", "grupo.jsonl"]);

    assert_eq!(pulled.code, 0, "{}", pulled.stderr);
    assert_eq!(pulled.stdout, "250\t250\tgrupo.jsonl\n");
    // Three pages reach the total; a fourth would only repeat.
    let requests = fs.requests();
    let asked: Vec<&str> = requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    let route = "/service/search/fulltext/search?q.groupName=009990001&count=100";
    assert_eq!(
        asked,
        [
            format!("{route}&offset=0"),
            format!("{route}&offset=100"),
            format!("{route}&offset=200"),
        ]
    );
    // The third page brought 50 new images and 50 repeated ones.
    let mut ids = pulled_ids(&env.work_dir().join("grupo.jsonl"));
    assert_eq!(ids.len(), 250);
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 250);
    let coverage = env.coverage();
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0]["busca"], "grupo");
    assert_eq!(coverage[0]["consulta"], "009990001");
    assert_eq!(coverage[0]["resultado"], "lido");
    assert_eq!(coverage[0]["vistos"], 250);
    assert_eq!(coverage[0]["total"], 250);
    assert_eq!(coverage[0]["arquivo"], "grupo.jsonl");
}

#[test]
fn an_interrupted_pull_is_recorded_as_partial_and_resumes_from_the_file() {
    let fs = group_of_250();
    // The session cap stops the pull after two of its three pages.
    let env = TestEnv::new(&fs).with_var("GARIMPO_TETO_SESSAO", "2");
    env.deliver_session();
    let pull = ["busca", "grupo", "009990001", "--saida", "grupo.jsonl"];

    let interrupted = env.run(&pull);
    env.deliver_session();
    let resumed = env.run(&pull);

    assert_eq!(interrupted.code, 5, "{}", interrupted.stderr);
    assert_eq!(resumed.code, 0, "{}", resumed.stderr);
    assert_eq!(resumed.stdout, "250\t250\tgrupo.jsonl\n");
    // The pages already in the file are not asked for again.
    let requests = fs.requests();
    let offsets: Vec<String> = requests
        .iter()
        .map(|request| request.param("offset").unwrap())
        .collect();
    assert_eq!(offsets, ["0", "100", "200"]);
    assert_eq!(pulled_ids(&env.work_dir().join("grupo.jsonl")).len(), 250);
    let coverage = env.coverage();
    let outcomes: Vec<(&str, u64)> = coverage
        .iter()
        .map(|record| {
            (
                record["resultado"].as_str().unwrap(),
                record["vistos"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(outcomes, [("parcial", 200), ("lido", 250)]);
}

#[test]
fn names_are_found_in_a_pulled_group_and_an_absent_name_is_a_negative() {
    let fs = FakeServer::start(|_| {
        full_text(
            3,
            &[
                // A line break in the middle of the name, as transcriptions have.
                page_image("SINT-001", "herdeiros: Joaquim Fictício\nda Silva, genro"),
                page_image("SINT-002", "a rogo de JOAQUIM FICTICIO DA SILVA"),
                page_image("SINT-003", "termo de encerramento"),
            ],
        )
    });
    let env = TestEnv::logged_in(&fs);
    let pulled = env.run(&["busca", "grupo", "009990001", "--saida", "grupo.jsonl"]);
    assert_eq!(pulled.code, 0, "{}", pulled.stderr);

    let found = env.run(&[
        "busca",
        "nomes",
        "grupo.jsonl",
        "Joaquim Fictício da Silva",
        "Maria Inventada",
    ]);

    assert_eq!(found.code, 0, "{}", found.stderr);
    assert_eq!(
        found.stdout,
        "Joaquim Fictício da Silva\tark:/61903/3:1:SINT-001\tVila Exemplo, Inventário 1850\t\
         herdeiros: Joaquim Fictício da Silva, genro\n\
         Joaquim Fictício da Silva\tark:/61903/3:1:SINT-002\tVila Exemplo, Inventário 1850\t\
         a rogo de JOAQUIM FICTICIO DA SILVA\n"
    );
    assert!(
        found
            .stderr
            .contains("sem ocorrência de \"Maria Inventada\" nas 3 imagens"),
        "{}",
        found.stderr
    );
    // The local search calls nothing: the pull made the only request.
    assert_eq!(fs.requests().len(), 1);
    let coverage = env.coverage();
    assert_eq!(coverage.len(), 3);
    assert_eq!(coverage[1]["busca"], "nomes");
    assert_eq!(
        coverage[1]["consulta"],
        "\"Joaquim Fictício da Silva\" no grupo 009990001"
    );
    assert_eq!(coverage[1]["resultado"], "lido");
    assert_eq!(
        coverage[1]["itens"],
        serde_json::json!(["ark:/61903/3:1:SINT-001", "ark:/61903/3:1:SINT-002"])
    );
    assert_eq!(coverage[2]["resultado"], "negativo");
    assert_eq!(coverage[2]["total"], 0);
}

#[test]
fn searches_that_run_at_the_same_time_each_leave_a_whole_line_in_the_coverage() {
    let fs = FakeServer::start(|_| full_text(1, &[page_image("SINT-001", "auto de partilha")]));
    let env = TestEnv::logged_in(&fs);
    let pulled = env.run(&["busca", "grupo", "009990001", "--saida", "grupo.jsonl"]);
    assert_eq!(pulled.code, 0, "{}", pulled.stderr);

    let searches: Vec<_> = (0..16)
        .map(|n| {
            let name = format!("Nome Sintético {n}");
            env.command(&["busca", "nomes", "grupo.jsonl", &name])
                .spawn()
                .unwrap()
        })
        .collect();

    for search in searches {
        let search = output(search);
        assert_eq!(search.code, 0, "{}", search.stderr);
    }
    // Reading the coverage parses every line: none was cut or mixed.
    assert_eq!(env.coverage().len(), 17);
}
