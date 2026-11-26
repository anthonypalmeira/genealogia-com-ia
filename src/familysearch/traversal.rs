//! Breadth-first ancestry and descendancy, in batches, saved to a file that
//! allows resuming. Without the 1,000-person cut of the ready-made routes.
//! The file is read back here too, by the commands that answer from it.

use std::{
    collections::{HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::Path,
};

use anyhow::Context;
use gedcomx::{FactType, Person};
use serde::{Deserialize, Serialize};

use super::{BATCH, Gedcomx, PersonId, person_id};
use crate::{cli::Config, dates::Date, error::Error, familysearch::field, transport::Transport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Direction {
    #[serde(rename = "ascendencia")]
    Ancestry,
    #[serde(rename = "descendencia")]
    Descendancy,
}

/// The first line of the file: which traversal it belongs to.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Header {
    #[serde(rename = "direcao")]
    pub(super) direction: Direction,
    #[serde(rename = "raiz")]
    pub(super) root: PersonId,
    /// The cut year decides whom the traversal follows, and with it the
    /// generation each person is first reached at: a file continues only
    /// under the cut year it began with.
    #[serde(rename = "ano-corte", default, skip_serializing_if = "Option::is_none")]
    cut_year: Option<i32>,
}

/// One visited person, with what it takes to continue from it.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Line {
    #[serde(rename = "geracao")]
    generation: u32,
    #[serde(rename = "pais")]
    pub(super) parents: Vec<PersonId>,
    #[serde(rename = "filhos")]
    children: Vec<PersonId>,
    #[serde(rename = "pessoa")]
    pub(super) person: Person,
}

/// What a traversal file holds.
struct Saved {
    /// Its whole lines. A run interrupted in the middle of a write leaves the
    /// last line cut short, maybe in the middle of a character: only whole
    /// lines are read.
    lines: String,
    /// The length of the file in bytes, with the line cut short if there is
    /// one. A file that does not exist is empty.
    length: usize,
}

fn saved(file: &Path) -> Result<Saved, Error> {
    let context = || format!("não consegui usar o arquivo {}", file.display());
    let mut bytes = match fs::read(file) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(anyhow::Error::new(error).context(context()).into()),
    };
    let length = bytes.len();
    let whole = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |end| end + 1);
    bytes.truncate(whole);
    Ok(Saved {
        lines: String::from_utf8(bytes).with_context(context)?,
        length,
    })
}

/// A traversal read back from its file.
#[derive(Debug)]
pub(super) struct Traversal {
    pub(super) header: Header,
    /// The persons, in the order they were visited.
    pub(super) lines: Vec<Line>,
}

impl Traversal {
    /// Reads the file a traversal wrote, complete or not.
    pub(super) fn read(file: &Path) -> Result<Self, Error> {
        let saved = saved(file)?;
        let mut lines = saved.lines.lines();
        let header = lines
            .next()
            .and_then(|first| serde_json::from_str(first).ok())
            .ok_or_else(|| {
                Error::Input(format!(
                    "{} não é o arquivo de uma varredura: gere-o com `garimpo fs ascendencia`",
                    file.display()
                ))
            })?;
        let lines = lines
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .with_context(|| format!("não consegui ler a varredura {}", file.display()))?;
        Ok(Self { header, lines })
    }
}

/// Prints the persons of a traversal file, one per line: id, generation,
/// every name, then one field per fact.
pub(super) fn list(config: &Config, file: &Path) -> Result<(), Error> {
    for line in Traversal::read(file)?.lines {
        let person = &line.person;
        let names = person
            .names
            .iter()
            .flat_map(|name| &name.name_forms)
            .filter_map(|form| form.full_text.as_deref())
            .collect::<Vec<_>>()
            .join(" / ");
        let facts = person.facts.iter().map(|fact| {
            let kind = fact.fact_type.to_string();
            let date = fact.date.as_ref();
            let formal = date
                .and_then(|date| date.formal.as_ref())
                .map(ToString::to_string);
            let date = match formal.as_deref() {
                // Keep the GEDCOM X notation for ranges and recurring dates.
                Some(text) if text.contains('/') => Some(text),
                Some(text) => Some(text.trim_start_matches('+')),
                None => date.and_then(|date| date.original.as_deref()),
            };
            let place = fact
                .place
                .as_ref()
                .and_then(|place| place.original.as_deref());
            // The type as GEDCOM X names it, without the address before it.
            let parts = [kind.rsplit('/').next(), fact.value.as_deref(), date, place];
            field(Some(
                parts.into_iter().flatten().collect::<Vec<_>>().join(" "),
            ))
        });
        let short = [
            field(person.id.as_ref()),
            line.generation.to_string(),
            field(Some(names)),
        ]
        .into_iter()
        .chain(facts)
        .collect::<Vec<_>>()
        .join("\t");
        config.print(&short, &line)?;
    }
    Ok(())
}

/// The persons still to visit, each with its generation.
struct Queue {
    direction: Direction,
    /// The last generation whose persons go in, if there is one.
    generations: Option<u32>,
    /// Nobody is followed past a person born before this year.
    cut_year: Option<i32>,
    pending: VecDeque<(PersonId, u32)>,
    seen: HashSet<PersonId>,
}

impl Queue {
    /// Queues the relatives of a visited person that were not seen yet, when
    /// they fit the limits. A person christened before the cut year was born
    /// before it too. A person with neither date in the tree is not known to
    /// be past the cut year, so the traversal goes on through it.
    fn follow(&mut self, line: &Line) {
        let last_generation = self.generations.is_some_and(|last| line.generation >= last);
        let past_cut = self.cut_year.is_some_and(|cut| {
            let date = |kind| Date::of(&line.person, &kind);
            let born = date(FactType::Birth).or_else(|| date(FactType::Christening));
            born.is_some_and(|born| born.year() < cut)
        });
        if last_generation || past_cut {
            return;
        }
        let next = match self.direction {
            Direction::Ancestry => &line.parents,
            Direction::Descendancy => &line.children,
        };
        for id in next {
            if self.seen.insert(id.clone()) {
                self.pending.push_back((id.clone(), line.generation + 1));
            }
        }
    }
}

/// Walks the tree from `root` and appends each person to `output`. If the file
/// already exists, continues from where it stopped. Returns how many persons
/// the file holds in the end.
pub(super) fn traverse(
    transport: &Transport,
    direction: Direction,
    root: &PersonId,
    generations: Option<u32>,
    cut_year: Option<i32>,
    output: &Path,
) -> Result<usize, Error> {
    let context = || format!("não consegui usar o arquivo {}", output.display());
    let header = &Header {
        direction,
        root: root.clone(),
        cut_year,
    };
    let mut queue = Queue {
        direction,
        generations,
        cut_year,
        pending: VecDeque::from([(root.clone(), 0)]),
        seen: HashSet::from([root.clone()]),
    };
    let existing = saved(output)?;
    let mut lines = existing.lines.lines();
    let first = lines.next();
    let same_traversal = first
        .is_some_and(|first| serde_json::from_str::<Header>(first).ok().as_ref() == Some(header));
    // Any other non-empty file is refused before it is touched.
    if existing.length > 0 && !same_traversal {
        return Err(Error::Input(format!(
            "{} não é desta varredura (outra pessoa, outra direção ou outro --ano-corte); use \
             outro arquivo em --saida",
            output.display()
        )));
    }
    let mut saved = HashSet::new();
    for text in lines {
        let line: Line = serde_json::from_str(text).with_context(context)?;
        queue.follow(&line);
        saved.extend(person_id(&line.person));
    }
    queue.pending.retain(|(id, _)| !saved.contains(id));

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(output)
        .with_context(context)?;
    if existing.lines.len() < existing.length {
        // The cut-short tail is dropped, and its persons are requested again.
        let whole = u64::try_from(existing.lines.len()).expect("a file length fits in 64 bits");
        file.set_len(whole).with_context(context)?;
    }
    if first.is_none() {
        let text = serde_json::to_string(&header).context("não consegui gerar o cabeçalho")?;
        writeln!(file, "{text}").with_context(context)?;
    }

    // One generation at a time, so a person is always reached first by its
    // shortest path and its generation is the right one.
    while let Some(&(_, generation)) = queue.pending.front() {
        let size = queue
            .pending
            .iter()
            .take(BATCH)
            .take_while(|(_, other)| *other == generation)
            .count();
        let mut batch: Vec<PersonId> = queue.pending.drain(..size).map(|(id, _)| id).collect();
        let gedcomx = Gedcomx::fetch_persons(transport, &batch)?;
        let links = gedcomx.parent_child_links()?;
        let mut text = String::new();
        for person in gedcomx.persons {
            // Only requested persons go in.
            let returned = person_id(&person);
            let Some(position) = batch.iter().position(|id| Some(id) == returned.as_ref()) else {
                continue;
            };
            let id = batch.swap_remove(position);
            let line = Line {
                generation,
                parents: links
                    .iter()
                    .filter(|link| link.child == id)
                    .map(|link| link.parent.clone())
                    .collect(),
                children: links
                    .iter()
                    .filter(|link| link.parent == id)
                    .map(|link| link.child.clone())
                    .collect(),
                person,
            };
            queue.follow(&line);
            text.push_str(
                &serde_json::to_string(&line).context("não consegui gerar a linha da pessoa")?,
            );
            text.push('\n');
            saved.insert(id);
        }
        file.write_all(text.as_bytes()).with_context(context)?;
        // What is left in the batch was requested and did not come back: it
        // stays out of the file, but not silently.
        if !batch.is_empty() {
            let missing: Vec<String> = batch.iter().map(ToString::to_string).collect();
            eprintln!(
                "garimpo: o FamilySearch não devolveu {} (apagadas, fundidas ou restritas); \
                 ficaram fora do arquivo",
                missing.join(", ")
            );
        }
    }
    Ok(saved.len())
}
