use std::{collections::HashSet, fs, io::Write, path::Path};

use serde_json::{Value, json};

use crate::{
    env::{TestEnv, output},
    server::{FakeServer, Reply, Request, person, pids, reference},
};

/// A synthetic tree in which person `n` has the children `3n+1`, `3n+2` and
/// `3n+3`, up to `total` persons. Answers like the batch persons route, with
/// the families in the `childAndParentsRelationships` extension.
fn ternary_descendancy(request: &Request, total: usize) -> Reply {
    let id = |n: usize| format!("SINT-{n}");
    let mut persons = Vec::new();
    let mut families = Vec::new();
    for pid in pids(request) {
        let n: usize = pid.strip_prefix("SINT-").unwrap().parse().unwrap();
        persons.push(person(&pid));
        if n > 0 {
            families
                .push(json!({"parent1": reference(&id((n - 1) / 3)), "child": reference(&pid)}));
        }
        for child in (3 * n + 1..=3 * n + 3).filter(|child| *child < total) {
            families.push(json!({"parent1": reference(&pid), "child": reference(&id(child))}));
        }
    }
    Reply::json(
        200,
        json!({"persons": persons, "childAndParentsRelationships": families}).to_string(),
    )
}

fn lines(file: &Path) -> Vec<Value> {
    fs::read_to_string(file)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn descendancy_passes_a_thousand_persons_in_batches_and_resumes_where_it_stopped() {
    // 1 + 3 + 9 + 27 + 81 + 243 + 729 persons in seven generations.
    const TOTAL: usize = 1093;
    let fs = FakeServer::start(|request| ternary_descendancy(request, TOTAL));
    let env = TestEnv::new(&fs).with_var("GARIMPO_TETO_SESSAO", "4");
    env.deliver_session();
    let file = env.work_dir().join("descendencia.jsonl");
    let command = [
        "fs",
        "descendencia",
        "SINT-0",
        "--geracoes",
        "6",
        "--saida",
        file.to_str().unwrap(),
    ];

    // The first run hits the session cap after four batches.
    let interrupted = env.run(&command);

    assert_eq!(interrupted.code, 5, "{}", interrupted.stderr);
    assert_eq!(lines(&file).len(), 1 + 1 + 3 + 9 + 27);
    // A run killed in the middle of a write would leave the last line cut short.
    let mut cut_short = fs::OpenOptions::new().append(true).open(&file).unwrap();
    cut_short
        .write_all(br#"{"geracao":4,"pais":["SINT-"#)
        .unwrap();

    // With a new session, the same command continues from the file.
    env.deliver_session();
    let complete = output(
        env.command(&command)
            .env("GARIMPO_TETO_SESSAO", "100")
            .spawn()
            .unwrap(),
    );

    assert_eq!(complete.code, 0, "{}", complete.stderr);
    assert_eq!(complete.stdout, format!("{TOTAL}\t{}\n", file.display()));
    let saved = lines(&file);
    assert_eq!(
        saved[0],
        json!({"direcao": "descendencia", "raiz": "SINT-0"})
    );
    let ids: HashSet<&str> = saved[1..]
        .iter()
        .map(|line| line["pessoa"]["id"].as_str().unwrap())
        .collect();
    assert_eq!((saved.len() - 1, ids.len()), (TOTAL, TOTAL));
    assert_eq!(saved[2]["geracao"], 1);
    assert_eq!(saved[2]["pais"], json!(["SINT-0"]));
    // Nobody was requested twice, and each generation went in batches of up to
    // 200 persons: 1, 3, 9, 27, 81, then 243 and 729 split.
    let batches: Vec<usize> = fs.requests().iter().map(|r| pids(r).len()).collect();
    assert_eq!(batches, [1, 3, 9, 27, 81, 200, 43, 200, 200, 200, 129]);
}

#[test]
fn ancestry_climbs_through_the_parents_up_to_the_generation_limit() {
    // A is the child of B and C; B is the child of D. The relationships come in
    // the standard GEDCOM X form (`ParentChild`).
    let fs = FakeServer::start(|request| {
        let requested = pids(request);
        let relationships: Vec<Value> = [("B", "A"), ("C", "A"), ("D", "B")]
            .into_iter()
            .filter(|(parent, child)| requested.iter().any(|pid| pid == parent || pid == child))
            .map(|(parent, child)| {
                json!({
                    "type": "http://gedcomx.org/ParentChild",
                    "person1": reference(parent),
                    "person2": reference(child),
                })
            })
            .collect();
        let persons: Vec<Value> = requested.iter().map(|pid| person(pid)).collect();
        Reply::json(
            200,
            json!({"persons": persons, "relationships": relationships}).to_string(),
        )
    });
    let env = TestEnv::logged_in(&fs);
    let file = env.work_dir().join("ascendencia.jsonl");

    let climb = env.run(&[
        "fs",
        "ascendencia",
        "A",
        "--geracoes",
        "1",
        "--saida",
        file.to_str().unwrap(),
        "--json",
    ]);

    assert_eq!(climb.code, 0, "{}", climb.stderr);
    assert_eq!(
        serde_json::from_str::<Value>(&climb.stdout).unwrap(),
        json!({"pessoas": 3, "arquivo": file})
    );
    let saved = lines(&file);
    let summary: Vec<(&str, u64, &Value, &Value)> = saved[1..]
        .iter()
        .map(|line| {
            (
                line["pessoa"]["id"].as_str().unwrap(),
                line["geracao"].as_u64().unwrap(),
                &line["pais"],
                &line["filhos"],
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("A", 0, &json!(["B", "C"]), &json!([])),
            ("B", 1, &json!(["D"]), &json!(["A"])),
            ("C", 1, &json!([]), &json!(["A"])),
        ]
    );
}

#[test]
fn a_file_that_is_not_this_traversal_is_refused_and_left_alone() {
    let fs = FakeServer::start(|_| Reply::empty(204));
    let env = TestEnv::logged_in(&fs);
    let file = env.work_dir().join("notas-do-usuario.txt");
    fs::write(&file, "anotações que não são uma varredura").unwrap();

    let refused = env.run(&["fs", "ascendencia", "A", "--saida", file.to_str().unwrap()]);

    assert_eq!(refused.code, 2, "{}", refused.stderr);
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "anotações que não são uma varredura"
    );
    assert_eq!(fs.requests().len(), 0);
}
