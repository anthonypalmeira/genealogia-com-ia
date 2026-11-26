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
        let persons = ["A", "B", "C", "D"].map(person);
        tree(request, &persons, &[("B", "A"), ("C", "A"), ("D", "B")])
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

/// The one dated fact of a synthetic person, as the tree routes send it.
fn dated(kind: &str, date: Value) -> Value {
    json!([{"type": format!("http://gedcomx.org/{kind}"), "date": date}])
}

/// Answers the batch persons route for a tree given as its persons and its
/// `(parent, child)` links, in the standard GEDCOM X form.
pub(crate) fn tree(request: &Request, persons: &[Value], links: &[(&str, &str)]) -> Reply {
    let requested = pids(request);
    let asked = |id: &str| requested.iter().any(|pid| pid == id);
    let persons: Vec<&Value> = persons
        .iter()
        .filter(|person| asked(person["id"].as_str().unwrap()))
        .collect();
    let relationships: Vec<Value> = links
        .iter()
        .filter(|(parent, child)| asked(parent) || asked(child))
        .map(|(parent, child)| {
            json!({
                "type": "http://gedcomx.org/ParentChild",
                "person1": reference(parent),
                "person2": reference(child),
            })
        })
        .collect();
    Reply::json(
        200,
        json!({"persons": persons, "relationships": relationships}).to_string(),
    )
}

#[test]
fn ancestry_with_a_cut_year_climbs_until_somebody_born_before_it() {
    // One line of twelve persons, each the child of the next, born 25 years
    // apart: P0 in 2000, P8 in 1800, P9 in 1775. P3 has no date, P5 has a
    // range for one, and of P9 the tree only knows the christening.
    let fs = FakeServer::start(|request| {
        let persons: Vec<Value> = (0..12)
            .map(|n| {
                let mut person = person(&format!("P{n}"));
                let year = 2000 - 25 * n;
                person["facts"] = match n {
                    3 => json!([]),
                    5 => dated(
                        "Birth",
                        json!({"original": "antes de 1700", "formal": "/+1700"}),
                    ),
                    // Only what the contributor typed, with no standard form.
                    9 => dated(
                        "Christening",
                        json!({"original": format!("cerca de {year}")}),
                    ),
                    _ => dated(
                        "Birth",
                        json!({"original": "5 de março", "formal": format!("+{year}-03-05")}),
                    ),
                };
                person
            })
            .collect();
        let ids: Vec<String> = (0..12).map(|n| format!("P{n}")).collect();
        let links: Vec<(&str, &str)> = (0..11)
            .map(|n| (ids[n + 1].as_str(), ids[n].as_str()))
            .collect();
        tree(request, &persons, &links)
    });
    let env = TestEnv::logged_in(&fs);
    let file = env.work_dir().join("ascendencia.jsonl");

    let climb = env.run(&[
        "fs",
        "ascendencia",
        "P0",
        "--ano-corte",
        "1800",
        "--saida",
        file.to_str().unwrap(),
    ]);

    assert_eq!(climb.code, 0, "{}", climb.stderr);
    // Past the eight generations of the default, through the persons with no
    // exact date and the one born in the cut year, and no further than the
    // first person born before it, who is in the file.
    let saved = lines(&file);
    let ids: Vec<&str> = saved[1..]
        .iter()
        .map(|line| line["pessoa"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["P0", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9"]
    );
    assert_eq!(climb.stdout, format!("10\t{}\n", file.display()));
    let asked: Vec<Vec<String>> = fs.requests().iter().map(pids).collect();
    assert!(!asked.concat().contains(&"P10".to_owned()), "{asked:?}");

    // Another cut year would follow other persons: it is another traversal,
    // and the file is left alone.
    let file = file.to_str().unwrap();
    let other = env.run(&[
        "fs",
        "ascendencia",
        "P0",
        "--ano-corte",
        "1700",
        "--saida",
        file,
    ]);

    assert_eq!(other.code, 2, "{}", other.stderr);
    assert_eq!(lines(Path::new(file)), saved);
    assert_eq!(fs.requests().len(), asked.len());
}

/// A synthetic family in which A reaches T twice: through B, and through C
/// and D. T has another child, X, who is not an ancestor of A.
///
/// B was born 11 years and 9 months after T, and C more than a year after D
/// died: by the years alone, 12 years and the year after, neither would show.
fn family(request: &Request) -> Reply {
    let dated = |id: &str, facts: Value| {
        let mut person = person(id);
        person["facts"] = facts;
        person
    };
    let fact = |kind: &str, formal: &str| json!({"type": format!("http://gedcomx.org/{kind}"), "date": {"formal": formal}});
    let mut t = dated(
        "T",
        json!([
            {
                "type": "http://gedcomx.org/Birth",
                "date": {"original": "1 de junho de 1800", "formal": "+1800-06-01"},
                "place": {"original": "Vila Exemplo, Minas Gerais"},
            },
            fact("Death", "A+1890"),
            {
                "type": "http://gedcomx.org/Occupation", "value": "lavrador",
                "date": {"original": "por volta de 1830"},
            },
        ]),
    );
    t["names"] = json!([
        {"nameForms": [{"fullText": "Pessoa Sintética T"}]},
        {"nameForms": [{"fullText": "Sintética T de Exemplo"}]},
    ]);
    let persons = [
        dated("A", json!([fact("Birth", "+1880-05-20")])),
        dated("B", json!([fact("Birth", "+1812-03-10")])),
        dated("C", json!([fact("Birth", "+1849-06-15")])),
        dated(
            "D",
            json!([fact("Birth", "+1820"), fact("Death", "+1848-02-01")]),
        ),
        t,
    ];
    let links = [
        ("B", "A"),
        ("C", "A"),
        ("T", "B"),
        ("D", "C"),
        ("T", "D"),
        ("T", "X"),
    ];
    tree(request, &persons, &links)
}

/// Traverses the ancestry of `root` into a file, as the agent does before
/// reading it.
pub(crate) fn ancestry_file(env: &TestEnv, root: &str) -> String {
    let file = env.work_dir().join("ascendencia.jsonl");
    let file = file.to_str().unwrap();
    let climb = env.run(&["fs", "ascendencia", root, "--saida", file]);
    assert_eq!(climb.code, 0, "{}", climb.stderr);
    file.to_owned()
}

#[test]
fn every_path_to_a_target_is_listed_with_the_impossible_dates_marked() {
    let fs = FakeServer::start(family);
    let env = TestEnv::logged_in(&fs);
    let file = ancestry_file(&env, "A");
    let traversed = fs.requests().len();

    let paths = env.run(&["fs", "caminhos", &file, "T", "U"]);
    let json = env.run(&["fs", "caminhos", &file, "T", "--json"]);

    assert_eq!(paths.code, 0, "{}", paths.stderr);
    let name = |id: &str| format!("Pessoa Sintética {id}");
    let link = |path: &str, child: [&str; 2], parent: [&str; 3], warning: &str| {
        [
            path,
            "T",
            child[0],
            &name(child[0]),
            child[1],
            parent[0],
            &name(parent[0]),
            parent[1],
            parent[2],
            warning,
        ]
        .join("\t")
    };
    let (a, b, c) = (
        ["A", "1880-05-20"],
        ["B", "1812-03-10"],
        ["C", "1849-06-15"],
    );
    let (d, t) = (["D", "1820"], ["T", "1800-06-01", "1890"]);
    assert_eq!(
        paths.stdout.lines().collect::<Vec<_>>(),
        [
            link("1", a, ["B", b[1], "-"], "-"),
            link(
                "1",
                b,
                t,
                "Pessoa Sintética T teria 11 anos quando Pessoa Sintética B nasceu"
            ),
            link("2", a, ["C", c[1], "-"], "-"),
            link(
                "2",
                c,
                ["D", d[1], "1848-02-01"],
                "Pessoa Sintética C nasceu depois da morte de Pessoa Sintética D"
            ),
            link("2", d, t, "-"),
        ]
    );
    // A target the file does not hold is told apart from one with no line.
    assert!(
        paths.stderr.contains("U não está no arquivo da varredura"),
        "{}",
        paths.stderr
    );
    assert_eq!(
        serde_json::from_str::<Value>(json.stdout.lines().nth(1).unwrap()).unwrap(),
        json!({
            "caminho": 1,
            "alvo": "T",
            "filho": {"id": "B", "nome": name("B"), "nascimento": "1812-03-10", "morte": null},
            "pai-ou-mae": {
                "id": "T", "nome": name("T"), "nascimento": "1800-06-01", "morte": "1890",
            },
            "avisos": ["Pessoa Sintética T teria 11 anos quando Pessoa Sintética B nasceu"],
        })
    );
    // The file answers: nothing more was asked of FamilySearch.
    assert_eq!(fs.requests().len(), traversed);
}

#[test]
fn the_persons_of_a_traversal_are_listed_with_every_name_and_fact() {
    let fs = FakeServer::start(family);
    let env = TestEnv::logged_in(&fs);
    let file = ancestry_file(&env, "A");

    let persons = env.run(&["fs", "varredura", &file]);

    assert_eq!(persons.code, 0, "{}", persons.stderr);
    assert_eq!(
        persons.stdout.lines().collect::<Vec<_>>(),
        [
            "A\t0\tPessoa Sintética A\tBirth 1880-05-20",
            "B\t1\tPessoa Sintética B\tBirth 1812-03-10",
            "C\t1\tPessoa Sintética C\tBirth 1849-06-15",
            "D\t2\tPessoa Sintética D\tBirth 1820\tDeath 1848-02-01",
            "T\t2\tPessoa Sintética T / Sintética T de Exemplo\t\
             Birth 1800-06-01 Vila Exemplo, Minas Gerais\tDeath 1890\t\
             Occupation lavrador por volta de 1830",
        ]
    );
}
