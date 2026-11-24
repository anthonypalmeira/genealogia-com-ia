//! `garimpo diagrama`: a small compiler from the description of a lineage to
//! a drawing that shows its own holes.
//!
//! The front end reads the description and lowers it to an intermediate form
//! in which every link points at persons that exist. The checks run over that
//! form and mark what breaks a rule. The back ends write the marked lineage
//! as Mermaid, as plain text or, through Pele, as a themed SVG.
//!
//! Inspired by the `statechart` skill of pstack, by Lauren Tan (MIT).

mod check;
mod mermaid;
mod plain;
mod svg;

use std::{
    collections::HashMap,
    fmt, fs,
    io::{self, Read},
    path::Path,
};

use anyhow::Context;
use serde::Deserialize;
use serde_json::json;

use self::check::Checked;
use crate::{
    cli::{Config, DiagramArgs, DiagramFormat},
    error::Error,
};

/// The grade of a link on the proof scale, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
enum Grade {
    A,
    B,
    C,
    D,
    F,
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// What a link is to the lineage being drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
enum Role {
    /// A link of the lineage itself: a child and the father or the mother.
    #[serde(rename = "direto")]
    Direct,
    /// A link to a relative off the line, used to prove it.
    #[serde(rename = "colateral")]
    Collateral,
    /// Anything else worth showing.
    #[serde(rename = "contexto")]
    Context,
}

/// The description of a lineage, as the agent writes it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Description {
    #[serde(rename = "titulo")]
    title: Option<String>,
    #[serde(rename = "pessoas")]
    persons: Vec<PersonEntry>,
    #[serde(rename = "elos")]
    links: Vec<LinkEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersonEntry {
    id: String,
    #[serde(rename = "nome")]
    name: String,
    #[serde(rename = "nascimento")]
    born: Option<i32>,
    #[serde(rename = "morte")]
    died: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkEntry {
    #[serde(rename = "de")]
    from: String,
    #[serde(rename = "para")]
    to: String,
    #[serde(rename = "relacao")]
    relation: Option<String>,
    #[serde(rename = "nota")]
    grade: Option<Grade>,
    #[serde(rename = "identidade")]
    identity: Option<Grade>,
    #[serde(rename = "documento")]
    document: Option<String>,
    #[serde(rename = "papel")]
    role: Role,
}

/// The intermediate form: a lineage whose links point at its own persons.
#[derive(Debug)]
struct Lineage {
    title: Option<String>,
    persons: Vec<Person>,
    links: Vec<Link>,
}

#[derive(Debug)]
struct Person {
    name: String,
    born: Option<i32>,
    died: Option<i32>,
}

impl Person {
    /// The years of the person, as `1820–1880`, `n. 1820` or `m. 1880`.
    fn years(&self) -> Option<String> {
        match (self.born, self.died) {
            (Some(born), Some(died)) => Some(format!("{born}–{died}")),
            (Some(born), None) => Some(format!("n. {born}")),
            (None, Some(died)) => Some(format!("m. {died}")),
            (None, None) => None,
        }
    }
}

/// One link: `from` is the child of `to`, or has to `to` the relation named.
#[derive(Debug)]
struct Link {
    /// The position of each person in `Lineage::persons`.
    from: usize,
    to: usize,
    /// `None` for the link of the proof scale: a child and a parent.
    relation: Option<String>,
    grade: Option<Grade>,
    /// The grade of "is this the same person?", when it was given apart.
    identity: Option<Grade>,
    document: Option<String>,
    role: Role,
}

impl Link {
    /// The grade the link is worth: the weaker of filiation and identity.
    fn worth(&self) -> Option<Grade> {
        self.grade
            .max(self.identity)
            .filter(|_| self.grade.is_some())
    }
}

impl TryFrom<Description> for Lineage {
    type Error = Error;

    fn try_from(description: Description) -> Result<Self, Error> {
        let mut positions = HashMap::new();
        for (position, person) in description.persons.iter().enumerate() {
            if positions.insert(person.id.as_str(), position).is_some() {
                return Err(Error::Input(format!("pessoa repetida: {}", person.id)));
            }
        }
        let position = |id: &str| {
            positions
                .get(id)
                .copied()
                .ok_or_else(|| Error::Input(format!("elo com pessoa desconhecida: {id}")))
        };
        let filled = |text: Option<String>| {
            text.map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|text| !text.is_empty())
        };
        let links = description
            .links
            .into_iter()
            .map(|link| {
                let (from, to) = (position(&link.from)?, position(&link.to)?);
                let relation = filled(link.relation);
                if from == to {
                    return Err(Error::Input(format!(
                        "elo de {} para a mesma pessoa",
                        link.from
                    )));
                }
                if link.role == Role::Direct && relation.is_some() {
                    return Err(Error::Input(format!(
                        "o elo direto de {} para {} é sempre filho e pai ou mãe: tire `relacao` \
                         ou mude o papel",
                        link.from, link.to
                    )));
                }
                Ok(Link {
                    from,
                    to,
                    relation,
                    grade: link.grade,
                    identity: link.identity,
                    document: filled(link.document),
                    role: link.role,
                })
            })
            .collect::<Result<_, Error>>()?;
        let persons = description
            .persons
            .into_iter()
            .map(|person| Person {
                name: person.name.split_whitespace().collect::<Vec<_>>().join(" "),
                born: person.born,
                died: person.died,
            })
            .collect();
        Ok(Self {
            title: filled(description.title),
            persons,
            links,
        })
    }
}

/// Runs `garimpo diagrama`: compiles the description and writes the drawing.
pub(crate) fn run(config: &Config, args: &DiagramArgs) -> Result<(), Error> {
    let source = if args.description == Path::new("-") {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .context("não consegui ler a descrição da entrada padrão")?;
        source
    } else {
        fs::read_to_string(&args.description).with_context(|| {
            format!(
                "não consegui ler a descrição {}",
                args.description.display()
            )
        })?
    };
    let description: Description = serde_json::from_str(&source)
        .map_err(|error| Error::Input(format!("a descrição da linhagem não serve: {error}")))?;
    let checked = Checked::new(Lineage::try_from(description)?);
    let drawing = match args.format {
        DiagramFormat::Mermaid => mermaid::draw(&checked),
        DiagramFormat::Text => plain::draw(&checked),
        DiagramFormat::Svg => svg::render(&mermaid::draw(&checked), args)?,
    };
    let holes = checked.holes();
    match holes.len() {
        0 => eprintln!("garimpo: nenhum furo na linhagem"),
        count => {
            eprintln!("garimpo: {count} furos na linhagem, marcados no desenho:");
            for hole in &holes {
                eprintln!("  {hole}");
            }
        }
    }
    match &args.output {
        Some(output) => {
            fs::write(output, &drawing)
                .with_context(|| format!("não consegui gravar {}", output.display()))?;
            config.print(
                &format!("{}\t{}", output.display(), holes.len()),
                &json!({"arquivo": output, "furos": holes}),
            )
        }
        None => config.print(
            drawing.trim_end_matches('\n'),
            &json!({"desenho": drawing, "furos": holes}),
        ),
    }
}
