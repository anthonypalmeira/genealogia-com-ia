use serde_json::json;

use crate::{
    env::TestEnv,
    server::{FakeServer, Reply, Request, person, pids, reference},
};

#[test]
fn person_prints_one_short_line_and_the_request_wears_no_disguise() {
    let fs = FakeServer::start(|_| Reply::recorded("person.json"));
    let env = TestEnv::logged_in(&fs);

    let person = env.run(&["fs", "pessoa", "SINT-001"]);

    assert_eq!(person.code, 0, "{}", person.stderr);
    assert_eq!(
        person.stdout,
        "SINT-001\tJoaquim Fictício da Silva\tM\t3 abr 1820\tVila Exemplo, Minas Gerais, Brasil\t\
         12 nov 1890\tVila Exemplo, Minas Gerais, Brasil\n"
    );
    let requests = fs.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].route(),
        ("GET", "/platform/tree/persons/SINT-001")
    );
    // No disguise: the client identifies itself as garimpo and sends no cookies.
    let agent = requests[0].header("User-Agent").unwrap();
    assert!(agent.starts_with("garimpo/"), "{agent}");
    assert_eq!(requests[0].header("Cookie"), None);
}

#[test]
fn with_json_each_item_is_one_line_of_gedcomx() {
    let fs = FakeServer::start(|_| Reply::recorded("parents.json"));
    let env = TestEnv::logged_in(&fs);

    let parents = env.run(&["fs", "pais", "SINT-001", "--json"]);

    assert_eq!(parents.code, 0, "{}", parents.stderr);
    let items: Vec<serde_json::Value> = parents
        .stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "SINT-002");
    assert_eq!(items[0]["gender"]["type"], "http://gedcomx.org/Male");
    assert_eq!(
        items[1]["names"][0]["nameForms"][0]["fullText"],
        "Maria Inventada de Jesus"
    );
}

#[test]
fn parents_list_one_person_per_line() {
    let fs = FakeServer::start(|_| Reply::recorded("parents.json"));
    let env = TestEnv::logged_in(&fs);

    let parents = env.run(&["fs", "pais", "SINT-001"]);

    assert_eq!(parents.code, 0, "{}", parents.stderr);
    assert_eq!(
        parents.stdout,
        "SINT-002\tManoel Fictício da Silva\tM\t1790\tVila Exemplo, Minas Gerais, Brasil\t\
         1850\tVila Exemplo, Minas Gerais, Brasil\n\
         SINT-003\tMaria Inventada de Jesus\tF\t1795\tVila Exemplo, Minas Gerais, Brasil\t-\t-\n"
    );
    let requests = fs.requests();
    let routes: Vec<(&str, &str)> = requests.iter().map(Request::route).collect();
    assert_eq!(routes, [("GET", "/platform/tree/persons/SINT-001/parents")]);
}

#[test]
fn children_and_spouses_named_only_by_relationships_are_read_in_a_batch() {
    // The children route answers with relationships and no person details; the
    // spouses route can name the spouse only in the couple relationship.
    let fs = FakeServer::start(|request| {
        let body = if request.path.ends_with("/children") {
            json!({"childAndParentsRelationships": [
                {"parent1": reference("SINT-001"), "child": reference("SINT-004")},
                {"parent1": reference("SINT-001"), "child": reference("SINT-005")},
            ]})
        } else if request.path.ends_with("/spouses") {
            json!({"relationships": [{
                "type": "http://gedcomx.org/Couple",
                "person1": reference("SINT-001"),
                "person2": reference("SINT-006"),
            }]})
        } else {
            let persons: Vec<_> = pids(request).iter().map(|pid| person(pid)).collect();
            json!({"persons": persons})
        };
        Reply::json(200, body.to_string())
    });
    let env = TestEnv::logged_in(&fs);

    let children = env.run(&["fs", "filhos", "SINT-001"]);
    let spouses = env.run(&["fs", "conjuges", "SINT-001"]);

    assert_eq!(children.code, 0, "{}", children.stderr);
    assert_eq!(
        children.stdout,
        "SINT-004\tPessoa Sintética SINT-004\t-\t-\t-\t-\t-\n\
         SINT-005\tPessoa Sintética SINT-005\t-\t-\t-\t-\t-\n"
    );
    assert_eq!(spouses.code, 0, "{}", spouses.stderr);
    assert_eq!(
        spouses.stdout,
        "SINT-006\tPessoa Sintética SINT-006\t-\t-\t-\t-\t-\n"
    );
    let requests = fs.requests();
    let routes: Vec<(&str, &str)> = requests.iter().map(Request::route).collect();
    assert_eq!(
        routes,
        [
            ("GET", "/platform/tree/persons/SINT-001/children"),
            ("GET", "/platform/tree/persons?pids=SINT-004,SINT-005"),
            ("GET", "/platform/tree/persons/SINT-001/spouses"),
            ("GET", "/platform/tree/persons?pids=SINT-006"),
        ]
    );
}

#[test]
fn sources_and_notes_print_one_per_line() {
    let fs = FakeServer::start(|request| {
        if request.path.ends_with("/sources") {
            Reply::recorded("sources.json")
        } else {
            Reply::recorded("notes.json")
        }
    });
    let env = TestEnv::logged_in(&fs);

    let sources = env.run(&["fs", "fontes", "SINT-001"]);
    let notes = env.run(&["fs", "notas", "SINT-001"]);

    assert_eq!(sources.code, 0, "{}", sources.stderr);
    assert_eq!(
        sources.stdout,
        "SINT-F01\tBatismo de Joaquim, 1820\t\
         https://exemplo.invalid/acervo/batismos/livro-3/folha-12\n"
    );
    assert_eq!(notes.code, 0, "{}", notes.stderr);
    // The line break in the text becomes a space, so the note fits one line.
    assert_eq!(
        notes.stdout,
        "Hipótese sobre os avós\tOs avós paternos podem estar no inventário de 1850. \
         Próximo passo: ler o título de herdeiros.\n"
    );
    let requests = fs.requests();
    assert_eq!(
        requests[0].route(),
        ("GET", "/platform/tree/persons/SINT-001/sources")
    );
    assert_eq!(
        requests[1].route(),
        ("GET", "/platform/tree/persons/SINT-001/notes")
    );
}

#[test]
fn empty_list_arrives_as_204_and_is_not_an_error() {
    let fs = FakeServer::start(|_| Reply::empty(204));
    let env = TestEnv::logged_in(&fs);

    let notes = env.run(&["fs", "notas", "SINT-001"]);
    let parents = env.run(&["fs", "pais", "SINT-001", "--json"]);

    assert_eq!(
        (notes.code, notes.stdout.as_str()),
        (0, ""),
        "{}",
        notes.stderr
    );
    assert_eq!(
        (parents.code, parents.stdout.as_str()),
        (0, ""),
        "{}",
        parents.stderr
    );
}

#[test]
fn invalid_identifier_is_an_input_error_and_makes_no_request() {
    let fs = FakeServer::start(|_| Reply::recorded("person.json"));
    let env = TestEnv::logged_in(&fs);

    let person = env.run(&["fs", "pessoa", "../users/current"]);

    assert_eq!(person.code, 2);
    assert!(
        person.stderr.contains("próximo passo: corrija a entrada"),
        "{}",
        person.stderr
    );
    assert_eq!(fs.requests().len(), 0);
}
