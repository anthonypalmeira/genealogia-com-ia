use std::fs;

use image::{GrayImage, ImageFormat};
use serde_json::{Value, json};

use crate::{
    env::TestEnv,
    images::{encoded, sha256},
    server::{FakeServer, Reply, Request},
};

/// A synthetic document id, in the shape digitarq gives.
const DOCUMENT: &str = "0123456789abcdef0123456789abcdef";

/// A value as the search wraps it.
fn wrapped(value: &str) -> Value {
    json!({"id": "sintetico", "value": value, "class": "E62_String"})
}

/// A synthetic description, as the search of digitarq sends it.
fn found(id: &str, code: &str, title: &str, files: u32) -> Value {
    json!({
        "id": id, "value": code, "archiveCode": "SINT",
        "referenceCode": wrapped(code),
        "dateInitial": wrapped("1753-01-01"), "dateFinal": wrapped("1755-12-31"),
        "titles": [wrapped(title)],
        "filesCount": files,
    })
}

fn routes(fs: &FakeServer) -> Vec<String> {
    fs.requests()
        .iter()
        .map(|request| request.path.clone())
        .collect()
}

#[test]
fn a_search_prints_one_line_per_description_and_enters_the_coverage_log() {
    let fs = FakeServer::start(|_| {
        let results = [
            found(
                DOCUMENT,
                "PT/SINT/HAB/001",
                "Habilitação de Joaquim Fictício",
                70,
            ),
            found(
                "fedcba",
                "PT/SINT/HAB/002",
                "Habilitação de Manoel Fictício",
                0,
            ),
        ];
        Reply::json(200, json!({"results": results, "total": 2}).to_string())
    });
    // No session: digitarq is public.
    let env = TestEnv::new(&fs);

    let searched = env.run(&["digitarq", "buscar", "habilitação Fictício", "--frase"]);

    assert_eq!(searched.code, 0, "{}", searched.stderr);
    assert_eq!(
        searched.stdout,
        format!(
            "{DOCUMENT}\tPT/SINT/HAB/001\t1753-01-01 a 1755-12-31\t\
             Habilitação de Joaquim Fictício\t70\n\
             fedcba\tPT/SINT/HAB/002\t1753-01-01 a 1755-12-31\tHabilitação de Manoel Fictício\t0\n"
        )
    );
    let requests = fs.requests();
    assert_eq!(
        requests[0].route(),
        (
            "GET",
            "/api/docs/search?query=habilita%C3%A7%C3%A3o+Fict%C3%ADcio&searchType=ALL_PHRASE\
             &fromIndex=0&max=20"
        )
    );
    assert_eq!(requests[0].header("Authorization"), None);
    let agent = requests[0].header("User-Agent").unwrap();
    assert!(agent.starts_with("garimpo/"), "{agent}");
    let coverage = env.coverage();
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0]["busca"], "digitarq-buscar");
    assert_eq!(coverage[0]["consulta"], "habilitação Fictício");
    assert_eq!(coverage[0]["resultado"], "lido");
    assert_eq!(coverage[0]["itens"], json!([DOCUMENT, "fedcba"]));
}

#[test]
fn a_document_prints_its_filled_fields_and_its_images_in_order() {
    let fs = FakeServer::start(|request| {
        if request.path.starts_with("/api/docs/frozen/") {
            Reply::json(
                200,
                json!({
                    "CompleteUnitId": "PT/SINT/HAB/001",
                    "UnitTitle": "Habilitação de Joaquim Fictício",
                    "ScopeContent": "Naturalidade: Vila Exemplo\r\nPais: Manoel Fictício e Maria Inventada",
                    "BiogHist": "", "ContainerTypeTermID": null, "UnitDateInitialCertainty": false,
                })
                .to_string(),
            )
        } else {
            let file = |id: &str, name: &str| json!({"id": id, "name": name, "type": "IMAGE", "representationID": "900"});
            let files = [
                file("5001", "PT-SINT-HAB-001_m0001.jpg"),
                file("5002", "PT-SINT-HAB-001_m0002.jpg"),
            ];
            Reply::json(
                200,
                json!({"results": files, "total": 2, "max": 2000, "fromIndex": 0}).to_string(),
            )
        }
    });
    let env = TestEnv::new(&fs);

    let document = env.run(&["digitarq", "documento", DOCUMENT]);
    let files = env.run(&["digitarq", "imagens", DOCUMENT]);

    assert_eq!(document.code, 0, "{}", document.stderr);
    // Only the fields with text, each on one line.
    assert_eq!(
        document.stdout,
        "CompleteUnitId\tPT/SINT/HAB/001\n\
         ScopeContent\tNaturalidade: Vila Exemplo Pais: Manoel Fictício e Maria Inventada\n\
         UnitTitle\tHabilitação de Joaquim Fictício\n"
    );
    assert_eq!(files.code, 0, "{}", files.stderr);
    assert_eq!(
        files.stdout,
        "1\t5001\tPT-SINT-HAB-001_m0001.jpg\t900\n2\t5002\tPT-SINT-HAB-001_m0002.jpg\t900\n"
    );
    assert_eq!(
        routes(&fs),
        [
            format!("/api/docs/frozen/{DOCUMENT}"),
            format!("/api/rdigital/{DOCUMENT}?fromIndex=0&max=2000"),
        ]
    );
    let kinds: Vec<_> = env
        .coverage()
        .iter()
        .map(|line| line["busca"].clone())
        .collect();
    assert_eq!(kinds, ["digitarq-documento", "digitarq-imagens"]);
}

/// The image the fake archive holds for a file: a TIFF, whatever its name says.
fn scan(file_id: &str) -> Vec<u8> {
    let width = if file_id == "5001" { 31 } else { 32 };
    encoded(GrayImage::new(width, 20), ImageFormat::Tiff)
}

#[test]
fn the_images_of_a_document_download_with_their_real_format_and_resume() {
    let fs = FakeServer::start(|request| match request.param("fileId") {
        Some(file_id) => Reply::bytes(200, "image/tiff", scan(&file_id)),
        None => {
            // Two digitizations of the same page, under the same name.
            let file = |id: &str, representation: &str| {
                json!({
                    "id": id, "name": "PT-SINT-HAB-001_m0001.jpg",
                    "representationID": representation,
                })
            };
            let files = [file("5001", "900"), file("5002", "901")];
            Reply::json(200, json!({"results": files, "total": 2}).to_string())
        }
    });
    let env = TestEnv::new(&fs);
    let download = ["digitarq", "baixar", DOCUMENT, "--pasta", "habilitacao"];

    let downloaded = env.run(&download);
    let again = env.run(&download);

    let group = format!("digitarq:{DOCUMENT}");
    assert_eq!(downloaded.code, 0, "{}", downloaded.stderr);
    assert_eq!(
        downloaded.stdout,
        format!("{group}\t2\t0\t0\thabilitacao\n")
    );
    let folder = env.work_dir().join("habilitacao");
    // The id of the file keeps the two apart.
    for id in ["5001", "5002"] {
        let name = format!("PT-SINT-HAB-001_m0001.{id}.tiff");
        assert_eq!(fs::read(folder.join(name)).unwrap(), scan(id), "{id}");
    }
    let manifest: Vec<Value> = fs::read_to_string(folder.join("manifesto.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        manifest[1],
        json!({
            "situacao": "baixada", "grupo": group, "numero": 1, "id": "5001",
            "arquivo": "PT-SINT-HAB-001_m0001.5001.tiff", "sha256": sha256(&scan("5001")),
            "bytes": scan("5001").len(), "largura": 31, "altura": 20,
        })
    );
    assert_eq!(again.stdout, format!("{group}\t0\t2\t0\thabilitacao\n"));
    // The second run lists the files again and downloads none.
    let downloads = |request: &&Request| request.param("fileId").is_some();
    assert_eq!(fs.requests().iter().filter(downloads).count(), 2);
    let coverage = env.coverage();
    assert_eq!(coverage[0]["busca"], "digitarq-baixar");
    assert_eq!(coverage[0]["resultado"], "lido");
    assert_eq!(coverage[0]["arquivo"], "habilitacao");
}

#[test]
fn a_block_ends_the_command_at_once_and_is_recorded_as_pending_access() {
    let fs = FakeServer::start(|_| Reply::empty(429).with_header("Retry-After", "1"));
    let env = TestEnv::new(&fs);

    let blocked = env.run(&["digitarq", "buscar", "Fictício"]);

    assert_eq!(blocked.code, 4, "{}", blocked.stderr);
    assert!(
        blocked.stderr.contains("o digitarq respondeu 429"),
        "{}",
        blocked.stderr
    );
    assert!(
        blocked.stderr.contains("pare de chamar o digitarq"),
        "{}",
        blocked.stderr
    );
    assert_eq!(fs.requests().len(), 1);
    let coverage = env.coverage();
    assert_eq!(coverage[0]["resultado"], "pendente-de-acesso");
    assert_eq!(coverage[0]["total"], Value::Null);
}

/// A page of the OAI-PMH endpoint, in the shape digitarq sends.
fn oai_page(records: &[(&str, &str, &str)], token: &str) -> Reply {
    let records: String = records
        .iter()
        .map(|(code, title, description)| {
            format!(
                "<record><header><identifier>oai:SINT:{code}</identifier><setSpec>SINT</setSpec>\
                 </header><metadata><oai_dc:dc xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
                 xmlns:oai_dc=\"http://www.openarchives.org/OAI/2.0/oai_dc/\">\
                 <dc:title>{title}</dc:title><dc:description>{description}</dc:description>\
                 <dc:date>1753-01-01</dc:date><dc:date>1755-12-31</dc:date>\
                 <dc:identifier>{code}</dc:identifier>\
                 <dc:identifier>https://digitarq.example/documentDetails/{code}</dc:identifier>\
                 </oai_dc:dc></metadata></record>"
            )
        })
        .collect();
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <OAI-PMH xmlns=\"http://www.openarchives.org/OAI/2.0/\">\
         <responseDate>2026-11-24T00:00:00Z</responseDate>\
         <request verb=\"ListRecords\">https://digitarq.example/oai</request>\
         <ListRecords>{records}<resumptionToken cursor=\"0\" completeListSize=\"3\">{token}\
         </resumptionToken></ListRecords></OAI-PMH>"
    );
    Reply::bytes(200, "text/xml; charset=utf-8", body.into_bytes())
}

#[test]
fn an_oai_harvest_continues_where_it_stopped_and_is_searched_for_names() {
    let fs = FakeServer::start(|request| match request.param("resumptionToken") {
        None => oai_page(
            &[
                (
                    "PT/SINT/HAB/001",
                    "Habilitação",
                    "Pais: Manoel Fictício e Maria Inventada",
                ),
                (
                    "PT/SINT/HAB/002",
                    "Habilitação de Joaquim Fictício",
                    "Original",
                ),
            ],
            "pagina-2",
        ),
        Some(_) => oai_page(&[("PT/SINT/HAB/003", "Inventário", "Sem nomes")], ""),
    });
    let env = TestEnv::new(&fs);
    let harvest = [
        "digitarq",
        "oai",
        "SINT",
        "--saida",
        "colheita.jsonl",
        "--desde",
        "2026-01-01",
        "--paginas",
        "1",
    ];

    let first = env.run(&harvest);
    let second = env.run(&harvest);
    let third = env.run(&harvest);
    let names = env.run(&["busca", "nomes", "colheita.jsonl", "Joaquim Ficticio"]);

    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(first.stdout, "2\t3\tcolheita.jsonl\n");
    assert!(
        first.stderr.contains("repita o mesmo comando"),
        "{}",
        first.stderr
    );
    assert_eq!(second.stdout, "3\t3\tcolheita.jsonl\n");
    // The harvest is over: nothing is asked again.
    assert_eq!(third.stdout, "3\t3\tcolheita.jsonl\n");
    assert_eq!(
        routes(&fs),
        [
            "/oai?verb=ListRecords&metadataPrefix=oai_dc&set=SINT&from=2026-01-01",
            "/oai?verb=ListRecords&resumptionToken=pagina-2",
        ]
    );
    assert_eq!(names.code, 0, "{}", names.stderr);
    assert_eq!(
        names.stdout,
        "Joaquim Ficticio\tPT/SINT/HAB/002\tHabilitação de Joaquim Fictício\t\
         Habilitação de Joaquim Fictício Original\n"
    );
    let results: Vec<_> = env
        .coverage()
        .iter()
        .map(|line| line["resultado"].clone())
        .collect();
    assert_eq!(results, ["parcial", "lido", "lido", "lido"]);
}
