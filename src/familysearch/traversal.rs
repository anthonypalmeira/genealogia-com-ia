//! Breadth-first ancestry and descendancy, in batches, saved to a file that
//! allows resuming. Without the 1,000-person cut of the ready-made routes.

use std::{
    collections::{HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::Path,
    str,
};

use anyhow::Context;
use gedcomx::Person;
use serde::{Deserialize, Serialize};

use super::{BATCH, Gedcomx, PersonId, person_id};
use crate::{error::Error, transport::Transport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Direction {
    #[serde(rename = "ascendencia")]
    Ancestry,
    #[serde(rename = "descendencia")]
    Descendancy,
}

/// The first line of the file: which traversal it belongs to.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Header {
    #[serde(rename = "direcao")]
    direction: Direction,
    #[serde(rename = "raiz")]
    root: PersonId,
}

/// One visited person, with what it takes to continue from it.
#[derive(Debug, Serialize, Deserialize)]
struct Line {
    #[serde(rename = "geracao")]
    generation: u32,
    #[serde(rename = "pais")]
    parents: Vec<PersonId>,
    #[serde(rename = "filhos")]
    children: Vec<PersonId>,
    #[serde(rename = "pessoa")]
    person: Person,
}

/// The persons still to visit, each with its generation.
struct Queue {
    direction: Direction,
    generations: u32,
    pending: VecDeque<(PersonId, u32)>,
    seen: HashSet<PersonId>,
}

impl Queue {
    /// Queues the relatives of a visited person that were not seen yet, when
    /// their generation fits the limit.
    fn follow(&mut self, line: &Line) {
        if line.generation >= self.generations {
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
    generations: u32,
    output: &Path,
) -> Result<usize, Error> {
    let context = || format!("não consegui usar o arquivo {}", output.display());
    let header = &Header {
        direction,
        root: root.clone(),
    };
    let mut queue = Queue {
        direction,
        generations,
        pending: VecDeque::from([(root.clone(), 0)]),
        seen: HashSet::from([root.clone()]),
    };
    let existing = match fs::read(output) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(anyhow::Error::new(error).context(context()).into()),
    };
    // A run interrupted in the middle of a write leaves the last line cut
    // short, maybe in the middle of a character: only whole lines are read.
    let whole = existing
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |end| end + 1);
    let mut lines = str::from_utf8(&existing[..whole])
        .with_context(context)?
        .lines();
    let first = lines.next();
    let same_traversal = first
        .is_some_and(|first| serde_json::from_str::<Header>(first).ok().as_ref() == Some(header));
    // Any other non-empty file is refused before it is touched.
    if !existing.is_empty() && !same_traversal {
        return Err(Error::Input(format!(
            "{} não é desta varredura; use outro arquivo em --saida",
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
    if whole < existing.len() {
        // The cut-short tail is dropped, and its persons are requested again.
        let length = u64::try_from(whole).expect("a file length fits in 64 bits");
        file.set_len(length).with_context(context)?;
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
