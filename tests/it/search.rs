use serde_json::json;

use crate::{
    env::TestEnv,
    server::{FakeServer, Reply, Request, full_text, page_image},
};

#[test]
fn full_text_prints_one_line_per_page_image_and_requires_the_phrase() {
    let fs = FakeServer::start(|_| {
        // The name sits 101 characters into the first page and 70 before its end.
        let long = format!(
            "{} Joaquim Fictício da Silva {}",
            "a".repeat(100),
            "b".repeat(100)
        );
        let mut matched = page_image("SINT-PAG-001", &long);
        matched["content"]["highlightTexts"] = json!(["Joaquim Ficticio"]);
        let short = page_image("SINT-PAG-002", "Auto de\npartilha dos bens");
        full_text(2, &[matched, short])
    });
    let env = TestEnv::logged_in(&fs);

    let found = env.run(&[
        "busca",
        "texto",
        "--frase",
        "Joaquim Fictício",
        "--lugar",
        "Vila Exemplo",
        "--de",
        "1840",
        "--ate",
        "1860",
    ]);

    assert_eq!(found.code, 0, "{}", found.stderr);
    // The passage keeps 80 characters on each side of the matched term.
    let passage = format!(
        "…{} Joaquim Fictício da Silva {}…",
        "a".repeat(79),
        "b".repeat(70)
    );
    assert_eq!(
        found.stdout,
        format!(
            "ark:/61903/3:1:SINT-PAG-001\tVila Exemplo, Inventário 1850\t\
             Coleção Sintética de Inventários\t{passage}\n\
             ark:/61903/3:1:SINT-PAG-002\tVila Exemplo, Inventário 1850\t\
             Coleção Sintética de Inventários\tAuto de partilha dos bens\n"
        )
    );
    assert!(
        found.stderr.contains("2 de 2 resultados"),
        "{}",
        found.stderr
    );
    let requests = fs.requests();
    assert_eq!(requests.len(), 1);
    // The phrase goes required and without accents.
    assert_eq!(
        requests[0].route(),
        (
            "GET",
            "/service/search/fulltext/search?q.text=%2B%22Joaquim+Ficticio%22\
             &q.anyPlace=Vila+Exemplo&q.anyDate.from=1840&q.anyDate.to=1860&count=20&offset=0"
        )
    );
    assert_eq!(requests[0].header("Accept"), Some("application/json"));
    // No disguise on the internal routes either.
    let agent = requests[0].header("User-Agent").unwrap();
    assert!(agent.starts_with("garimpo/"), "{agent}");
    assert_eq!(requests[0].header("Cookie"), None);
}

#[test]
fn a_quoted_phrase_without_the_plus_gets_a_warning() {
    let fs = FakeServer::start(|_| full_text(0, &[]));
    let env = TestEnv::logged_in(&fs);

    let found = env.run(&["busca", "texto", "\"Joaquim Ficticio\" +inventario"]);

    assert_eq!(found.code, 0, "{}", found.stderr);
    assert!(
        found.stderr.contains("use --frase \"Joaquim Ficticio\""),
        "{}",
        found.stderr
    );
    // The query still goes as it was written.
    assert_eq!(
        fs.requests()[0].param("q.text").as_deref(),
        Some("\"Joaquim Ficticio\" +inventario")
    );
}

#[test]
fn indexed_records_print_the_principal_person_and_the_others_of_the_record() {
    let fs = FakeServer::start(|_| Reply::recorded("personas-search.json"));
    let env = TestEnv::logged_in(&fs);

    let found = env.run(&[
        "busca",
        "registros",
        "--nome",
        "Joaquim",
        "--sobrenome",
        "Silva",
        "--pai",
        "Manoel",
        "--lugar",
        "Vila Exemplo",
        "--de",
        "1815",
        "--ate",
        "1825",
        "--exigir",
    ]);

    assert_eq!(found.code, 0, "{}", found.stderr);
    assert_eq!(
        found.stdout,
        "ark:/61903/1:1:SINT-R01\tJoaquim Fictício da Silva\tChristening\t3 abr 1820\t\
         Vila Exemplo, Minas Gerais, Brasil\tColeção Sintética de Batismos\t\
         Manoel Fictício da Silva; Maria Inventada de Jesus\n\
         ark:/61903/1:1:SINT-R02\tJoaquim Imaginado da Silva\t-\t-\t-\t-\t-\n"
    );
    assert_eq!(
        fs.requests()[0].route(),
        (
            "GET",
            "/service/search/hr/v2/personas?q.givenName=Joaquim&q.givenName.require=on\
             &q.surname=Silva&q.surname.require=on&q.fatherGivenName=Manoel\
             &q.fatherGivenName.require=on&q.anyPlace=Vila+Exemplo&q.anyPlace.require=on\
             &q.birthLikeDate.from=1815&q.birthLikeDate.to=1825&q.birthLikeDate.require=on\
             &count=20&offset=0"
        )
    );
}

#[test]
fn the_catalog_finds_an_item_and_lists_its_films_with_their_access() {
    let fs = FakeServer::start(|request| {
        if request.path == "/service/search/catalog/item/9990002" {
            Reply::recorded("catalog-item.json")
        } else if request
            .path
            .starts_with("/service/search/catalog/v3/search?")
        {
            Reply::recorded("catalog-search.json")
        } else {
            Reply::empty(404)
        }
    });
    let env = TestEnv::logged_in(&fs);

    let found = env.run(&[
        "busca",
        "catalogo",
        "--lugar",
        "Brasil, Minas Gerais, Vila Exemplo",
    ]);
    let item = env.run(&["busca", "catalogo-item", "9990002"]);

    assert_eq!(found.code, 0, "{}", found.stderr);
    assert_eq!(
        found.stdout,
        "9990002\tVila Exemplo, registros paroquiais, 1750-1900\t\
         Igreja Católica. Paróquia Sintética (Vila Exemplo)\n"
    );
    assert_eq!(item.code, 0, "{}", item.stderr);
    // The image group gets back the zeros the catalog drops.
    assert_eq!(
        item.stdout,
        "item\t9990002\tVila Exemplo, registros paroquiais, 1750-1900\t1750-1900\tManuscrito\n\
         filme\t009990001\t1234567\taberto\tBatismos 1750-1800, Item 1\n\
         filme\t009990002\t-\trestrito\tCasamentos 1800-1850\n"
    );
    let requests = fs.requests();
    let routes: Vec<(&str, &str)> = requests.iter().map(Request::route).collect();
    assert_eq!(
        routes,
        [
            (
                "GET",
                "/service/search/catalog/v3/search?query=%2Bplace%3A%22Brasil%2C+Minas+Gerais\
                 %2C+Vila+Exemplo%22&count=20&offset=0"
            ),
            ("GET", "/service/search/catalog/item/9990002"),
        ]
    );
}

#[test]
fn waypoints_list_the_levels_of_a_collection_and_the_images_of_a_volume() {
    let fs = FakeServer::start(|request| {
        if request.path.contains("/collections/") {
            Reply::recorded("waypoints-collection.json")
        } else {
            Reply::recorded("waypoints-volume.json")
        }
    });
    let env = TestEnv::logged_in(&fs);

    let levels = env.run(&["busca", "waypoints", "9990001"]);
    let images = env.run(&[
        "busca",
        "waypoints",
        "9990001",
        "SINT-V1:100,101",
        "--quantos",
        "2",
    ]);

    assert_eq!(levels.code, 0, "{}", levels.stderr);
    assert_eq!(
        levels.stdout,
        "waypoint\tSINT-W1:100\tVila Exemplo\nwaypoint\tSINT-W2:100\tArraial Inventado\n"
    );
    assert_eq!(images.code, 0, "{}", images.stderr);
    assert_eq!(
        images.stdout,
        "imagem\t1\tark:/61903/3:1:SINT-IMG-001\nimagem\t2\tark:/61903/3:1:SINT-IMG-002\n"
    );
    // The volume has one more image than the two that were asked for.
    assert!(
        images.stderr.contains("2 de 3 resultados"),
        "{}",
        images.stderr
    );
    let requests = fs.requests();
    let routes: Vec<(&str, &str)> = requests.iter().map(Request::route).collect();
    assert_eq!(
        routes,
        [
            (
                "GET",
                "/platform/records/collections/9990001/waypoints?count=100&start=0"
            ),
            (
                "GET",
                "/platform/records/waypoints/SINT-V1:100,101?count=2&start=0&cc=9990001"
            ),
        ]
    );
}

#[test]
fn with_the_internal_routes_off_a_search_exits_8_and_makes_no_request() {
    let fs = FakeServer::start(|_| Reply::recorded("current-user.json"));
    let env = TestEnv::new(&fs).with_var("GARIMPO_ROTAS_INTERNAS", "desligadas");
    env.deliver_session();

    let refused = env.run(&["busca", "texto", "+inventario"]);
    let official = env.run(&["sessao", "conferir"]);

    assert_eq!(refused.code, 8, "{}", refused.stderr);
    assert!(
        refused.stderr.contains("só o usuário religa"),
        "{}",
        refused.stderr
    );
    // The official routes go on working.
    assert_eq!(official.code, 0, "{}", official.stderr);
    let requests = fs.requests();
    let routes: Vec<(&str, &str)> = requests.iter().map(Request::route).collect();
    assert_eq!(routes, [("GET", "/platform/users/current")]);
}
