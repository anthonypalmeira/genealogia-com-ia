//! The paths from the person an ancestry traversal started at up to target
//! ancestors, read from its file, with the links whose dates cannot both be
//! true marked. The dates of the tree are a hypothesis: a mark points at a
//! date or a relationship to check, and the link stays on the path.

use std::{collections::HashMap, path::Path};

use gedcomx::{FactType, GenderType, Person};
use serde_json::{Value, json};

use super::{
    PersonId, field, name, person_id,
    traversal::{Direction, Line, Traversal},
};
use crate::{
    cli::Config,
    dates::{self, Date, Lifespan},
    error::Error,
};

/// Prints every path from the root of the traversal in `file` up to each
/// target, one line per link.
pub(super) fn list(config: &Config, file: &Path, targets: &[PersonId]) -> Result<(), Error> {
    let traversal = Traversal::read(file)?;
    if traversal.header.direction != Direction::Ancestry {
        return Err(Error::Input(format!(
            "{} é uma varredura de descendência; os caminhos sobem pelos pais: use a de \
             `garimpo fs ascendencia`",
            file.display()
        )));
    }
    let lines: Vec<(PersonId, &Line)> = traversal
        .lines
        .iter()
        .filter_map(|line| Some((person_id(&line.person)?, line)))
        .collect();
    let persons: HashMap<&PersonId, &Person> =
        lines.iter().map(|(id, line)| (id, &line.person)).collect();
    // Only the children the traversal holds: the tree also gives the ones
    // outside the line.
    let mut children: HashMap<&PersonId, Vec<&PersonId>> = HashMap::new();
    for (child, line) in &lines {
        for parent in &line.parents {
            children.entry(parent).or_default().push(child);
        }
    }
    let mut number = 0;
    for target in targets {
        if !persons.contains_key(target) {
            eprintln!("garimpo: {target} não está no arquivo da varredura");
            continue;
        }
        let mut paths = Vec::new();
        let root = &traversal.header.root;
        descend(&children, root, target, &mut Vec::new(), &mut paths);
        for path in paths {
            number += 1;
            for link in path.windows(2) {
                let (child, parent) = (
                    Summary::of(&persons, link[0]),
                    Summary::of(&persons, link[1]),
                );
                let warnings =
                    dates::impossible_together(child.lifespan(), parent.lifespan(), parent.gender);
                let short = [
                    number.to_string(),
                    target.to_string(),
                    child.id.to_string(),
                    field(child.name),
                    field(child.born),
                    parent.id.to_string(),
                    field(parent.name),
                    field(parent.born),
                    field(parent.died),
                    field(Some(warnings.join("; "))),
                ]
                .join("\t");
                config.print(
                    &short,
                    &json!({
                        "caminho": number,
                        "alvo": target,
                        "filho": child.json(),
                        "pai-ou-mae": parent.json(),
                        "avisos": warnings,
                    }),
                )?;
            }
        }
    }
    eprintln!("garimpo: {number} caminhos");
    Ok(())
}

/// Collects every path from `person` down to `root`, each one from the root
/// up. `above` holds the persons between the target and `person`: one of them
/// is not visited again, so a loop in the tree ends.
fn descend<'a>(
    children: &HashMap<&'a PersonId, Vec<&'a PersonId>>,
    root: &PersonId,
    person: &'a PersonId,
    above: &mut Vec<&'a PersonId>,
    paths: &mut Vec<Vec<&'a PersonId>>,
) {
    above.push(person);
    if person == root {
        paths.push(above.iter().rev().copied().collect());
    } else {
        for child in children.get(person).into_iter().flatten() {
            if !above.contains(child) {
                descend(children, root, child, above, paths);
            }
        }
    }
    above.pop();
}

/// What a line shows of one person of a path.
struct Summary<'a> {
    id: &'a PersonId,
    name: Option<&'a str>,
    gender: Option<&'a GenderType>,
    born: Option<Date>,
    died: Option<Date>,
}

impl<'a> Summary<'a> {
    /// The person of a path, which only holds persons of the traversal.
    fn of(persons: &HashMap<&PersonId, &'a Person>, id: &'a PersonId) -> Self {
        let person = persons[id];
        Self {
            id,
            name: name(person),
            gender: person.gender.as_ref().map(|gender| &gender.gender_type),
            born: Date::of(person, &FactType::Birth),
            died: Date::of(person, &FactType::Death),
        }
    }

    fn lifespan(&self) -> Lifespan<'a> {
        Lifespan {
            name: self.name.unwrap_or("?"),
            born: self.born,
            died: self.died,
        }
    }

    fn json(&self) -> Value {
        let date = |date: Option<Date>| date.map(|date| date.to_string());
        json!({
            "id": self.id,
            "nome": self.name,
            "nascimento": date(self.born),
            "morte": date(self.died),
        })
    }
}
