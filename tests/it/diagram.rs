use std::{env, fs};

use serde_json::{Value, json};

use crate::{env::TestEnv, server::FakeServer};

/// Joaquim, his father Manoel and his grandfather Antônio, with Joaquim's
/// brother José as a collateral and a godfather as context.
fn lineage() -> Value {
    json!({
        "titulo": "Linhagem de Joaquim até Antônio",
        "pessoas": [
            {"id": "joaquim", "nome": "Joaquim Fictício da Silva", "nascimento": 1820, "morte": 1880},
            {"id": "manoel", "nome": "Manoel Fictício da Silva", "nascimento": 1790, "morte": 1850},
            {"id": "antonio", "nome": "Antônio Inventado da Silva", "nascimento": 1760},
            {"id": "jose", "nome": "José Fictício da Silva", "nascimento": 1822},
            {"id": "padrinho", "nome": "Pedro Imaginado"},
        ],
        "elos": [
            {"de": "joaquim", "para": "manoel", "nota": "A", "papel": "direto",
             "documento": "Casamento de Joaquim, Vila Exemplo, 1845"},
            {"de": "manoel", "para": "antonio", "nota": "B", "identidade": "C", "papel": "direto",
             "documento": "Batismo de Manoel, Vila Exemplo, 1790"},
            {"de": "jose", "para": "manoel", "nota": "A", "papel": "colateral",
             "documento": "Inventário de Manoel, 1850"},
            {"de": "padrinho", "para": "joaquim", "relacao": "padrinho", "nota": "C",
             "papel": "contexto", "documento": "Batismo de Joaquim, 1820"},
        ],
    })
}

/// Runs `garimpo diagrama` on a description, with the given extra arguments.
fn draw(description: &Value, args: &[&str]) -> crate::env::Output {
    let fs = FakeServer::start(|_| unreachable!("the diagram makes no request"));
    let env = TestEnv::new(&fs);
    fs::write(
        env.work_dir().join("linhagem.json"),
        description.to_string(),
    )
    .unwrap();
    env.run(&[&["diagrama", "linhagem.json"], args].concat())
}

#[test]
fn a_lineage_becomes_mermaid_with_the_roles_the_grades_and_the_weakest_link() {
    let drawn = draw(&lineage(), &[]);

    assert_eq!(drawn.code, 0, "{}", drawn.stderr);
    assert_eq!(
        drawn.stdout,
        "\
---
title: \"Linhagem de Joaquim até Antônio\"
---
flowchart BT
    p0[\"Joaquim Fictício da Silva<br/>1820–1880\"]
    p1[\"Manoel Fictício da Silva<br/>1790–1850\"]
    p2[\"Antônio Inventado da Silva<br/>n. 1760\"]
    p3[\"José Fictício da Silva<br/>n. 1822\"]
    p4[\"Pedro Imaginado\"]
    p0 ==>|\"A · Casamento de Joaquim, Vila Exemplo, 1845\"| p1
    p1 ==>|\"▼ elo mais fraco · B · identidade C · Batismo de Manoel, Vila Exemplo, 1790\"| p2
    p3 -->|\"A · Inventário de Manoel, 1850\"| p1
    p4 -.->|\"padrinho · C · Batismo de Joaquim, 1820\"| p0
    classDef direto fill:#dbeafe,stroke:#1d4ed8,color:#111827
    classDef colateral fill:#f3f4f6,stroke:#6b7280,color:#111827
    classDef contexto fill:#ffffff,stroke:#9ca3af,color:#4b5563,stroke-dasharray:4 3
    classDef quebra fill:#fee2e2,stroke:#dc2626,color:#991b1b,stroke-width:3px
    class p0,p1,p2 direto
    class p3 colateral
    class p4 contexto
    linkStyle 0 stroke:#15803d,stroke-width:2px
    linkStyle 1 stroke:#b45309,stroke-width:5px
    linkStyle 2 stroke:#15803d,stroke-width:2px
    linkStyle 3 stroke:#b45309,stroke-width:2px
"
    );
    assert!(drawn.stderr.contains("nenhum furo"), "{}", drawn.stderr);
}

#[test]
fn a_broken_rule_shows_marked_in_the_drawing_and_listed_in_the_text() {
    let mut broken = lineage();
    // The link to the grandfather, the weakest of the line, loses its
    // document; the brother's link loses its grade; the grandfather is born
    // after his son; and somebody is tied to nobody.
    broken["elos"][1] = json!({"de": "manoel", "para": "antonio", "nota": "C", "papel": "direto"});
    broken["elos"][2] = json!({
        "de": "jose", "para": "manoel", "papel": "colateral",
        "documento": "Inventário de Manoel, 1850",
    });
    broken["pessoas"][2]["nascimento"] = json!(1795);
    broken["pessoas"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "solta", "nome": "Maria Inventada"}));

    let mermaid = draw(&broken, &[]);
    let text = draw(&broken, &["--formato", "texto"]);

    assert_eq!(mermaid.code, 0, "{}", mermaid.stderr);
    for marked in [
        // Broken and still pointed out as the weakest of the line.
        "p1 ==>|\"▼ elo mais fraco · ⚠ sem documento, Antônio Inventado da Silva teria -5 anos \
         quando Manoel Fictício da Silva nasceu\"| p2",
        "linkStyle 1 stroke:#dc2626,stroke-width:5px,stroke-dasharray:6 4",
        "p3 -->|\"⚠ sem nota\"| p1",
        "p5[\"Maria Inventada<br/>⚠ sem ligação com a linha direta\"]",
        "class p5 quebra",
        "linkStyle 2 stroke:#dc2626,stroke-width:4px,stroke-dasharray:6 4",
        "p0 ==>|\"A · Casamento de Joaquim, Vila Exemplo, 1845\"| p1",
    ] {
        assert!(
            mermaid.stdout.contains(marked),
            "{marked}\n{}",
            mermaid.stdout
        );
    }
    assert!(mermaid.stderr.contains("4 furos"), "{}", mermaid.stderr);

    assert_eq!(text.code, 0, "{}", text.stderr);
    assert_eq!(
        text.stdout,
        "\
Linhagem de Joaquim até Antônio

Linha direta
1. Joaquim Fictício da Silva (1820–1880)
   ↑ nota A — Casamento de Joaquim, Vila Exemplo, 1845
2. Manoel Fictício da Silva (1790–1850)
   ↑ ⚠ sem documento, Antônio Inventado da Silva teria -5 anos quando Manoel Fictício da \
Silva nasceu ◀ elo mais fraco
3. Antônio Inventado da Silva (n. 1795)

Colaterais e contexto
- José Fictício da Silva (n. 1822), filho(a) de Manoel Fictício da Silva: ⚠ sem nota
- Pedro Imaginado, padrinho de Joaquim Fictício da Silva: nota C — Batismo de Joaquim, 1820

Furos
- Manoel Fictício da Silva → Antônio Inventado da Silva: sem documento
- Manoel Fictício da Silva → Antônio Inventado da Silva: Antônio Inventado da Silva teria -5 \
anos quando Manoel Fictício da Silva nasceu
- José Fictício da Silva → Manoel Fictício da Silva: sem nota
- Maria Inventada: sem ligação com a linha direta
"
    );
}

#[test]
fn a_direct_line_that_splits_or_breaks_is_marked_as_disconnected() {
    let mut split = lineage();
    // A second direct link leaves Joaquim, another one floats by itself, and
    // a third one turns the line back on itself.
    split["pessoas"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "outro", "nome": "Outro Inventado"}));
    for (from, to) in [
        ("joaquim", "outro"),
        ("jose", "padrinho"),
        ("antonio", "manoel"),
    ] {
        split["elos"].as_array_mut().unwrap().push(json!({
            "de": from, "para": to, "nota": "A", "papel": "direto", "documento": "Documento",
        }));
    }

    let text = draw(&split, &["--formato", "texto"]);

    assert_eq!(text.code, 0, "{}", text.stderr);
    for hole in [
        "- Joaquim Fictício da Silva → Outro Inventado: fora da linha direta, que segue por \
         outro elo",
        "- José Fictício da Silva → Pedro Imaginado: fora da linha direta, que segue por outro \
         elo",
        "- Antônio Inventado da Silva → Manoel Fictício da Silva: fora da linha direta, que \
         segue por outro elo",
    ] {
        assert!(text.stdout.contains(hole), "{hole}\n{}", text.stdout);
    }
    // The three links off the line, and the son who would be older than the
    // father in the one that turns back.
    assert!(text.stderr.contains("4 furos"), "{}", text.stderr);
}

#[test]
fn a_link_to_an_unknown_person_is_an_input_error_and_draws_nothing() {
    let mut unknown = lineage();
    unknown["elos"][0]["para"] = json!("ninguem");

    let refused = draw(&unknown, &[]);

    assert_eq!(refused.code, 2, "{}", refused.stderr);
    assert_eq!(refused.stdout, "");
    assert!(
        refused.stderr.contains("pessoa desconhecida: ninguem"),
        "{}",
        refused.stderr
    );
}

#[test]
fn the_svg_is_the_same_drawing_rendered_by_pele_with_the_colors_of_the_theme() {
    let fs = FakeServer::start(|_| unreachable!("the diagram makes no request"));
    // The renderer runs on `bun`, found through the PATH of whoever runs the tests.
    let env = TestEnv::new(&fs).with_var("PATH", &env::var("PATH").unwrap());
    fs::write(env.work_dir().join("linhagem.json"), lineage().to_string()).unwrap();

    let drawn = env.run(&[
        "diagrama",
        "linhagem.json",
        "--formato",
        "svg",
        "--tema",
        "pergaminho",
        "--saida",
        "linhagem.svg",
    ]);

    assert_eq!(drawn.code, 0, "{}", drawn.stderr);
    assert_eq!(drawn.stdout, "linhagem.svg\t0\n");
    let svg = fs::read_to_string(env.work_dir().join("linhagem.svg")).unwrap();
    assert!(svg.starts_with("<svg "), "{}", &svg[..svg.len().min(200)]);
    for part in [
        // The parchment theme, and the weakest link of the line, 5 px wide.
        "<style>.pele{--pele-bg:#f5ecd7;",
        "Joaquim Fictício da Silva",
        "stroke:#b45309;stroke-width:5px",
    ] {
        assert!(svg.contains(part), "{part}");
    }
}
