//! The official FamilySearch tree routes (`/platform/tree`), read with the
//! types of the `gedcomx` crate.

mod paths;
mod portraits;
mod traversal;
mod write;

use std::{collections::HashSet, fmt, str::FromStr};

use anyhow::Context;
use gedcomx::{
    FactType, GenderType, Note, Person, Relationship, RelationshipType, ResourceReference,
    SourceDescription,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value, json};

use self::traversal::Direction;
use crate::{
    cli::{Config, FsCommand, TraversalArgs},
    error::Error,
    transport::Transport,
};

/// The most persons the batch route accepts in one request.
const BATCH: usize = 200;

/// How many generations a traversal covers when nothing else limits it.
const GENERATIONS: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("identificador inválido: use só letras, números e hífen, como KWXX-XXX")]
pub struct InvalidId;

/// Declares a FamilySearch identifier: only letters, digits and hyphens, so it
/// can go into a URL path unescaped. Each one is its own type, so a person id
/// cannot pass for a source id.
macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl FromStr for $name {
            type Err = InvalidId;

            fn from_str(text: &str) -> Result<Self, InvalidId> {
                let valid = !text.is_empty()
                    && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
                if valid { Ok(Self(text.to_owned())) } else { Err(InvalidId) }
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvalidId;

            fn try_from(text: String) -> Result<Self, InvalidId> {
                text.parse()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(
    /// The identifier of a person in the tree, such as `KWXX-XXX`.
    PersonId
);
id_type!(
    /// The identifier of a source (a source description), such as `MMMM-XXX`.
    SourceId
);

/// The GEDCOM X document of the tree responses. FamilySearch sends only the
/// keys that have content, and nothing at all (204) when a list is empty.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Gedcomx {
    persons: Vec<Person>,
    relationships: Vec<Relationship>,
    child_and_parents_relationships: Vec<ChildAndParents>,
    #[serde(deserialize_with = "lenient_source_descriptions")]
    source_descriptions: Vec<SourceDescription>,
}

/// Reads the source descriptions. FamilySearch omits empty lists, and a source
/// may have no citation, but the gedcomx `SourceDescription` requires
/// `citations`.
fn lenient_source_descriptions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<SourceDescription>, D::Error> {
    Vec::<Map<String, Value>>::deserialize(deserializer)?
        .into_iter()
        .map(|mut description| {
            description.entry("citations").or_insert_with(|| json!([]));
            serde_json::from_value(Value::Object(description)).map_err(serde::de::Error::custom)
        })
        .collect()
}

/// A child's family, a FamilySearch extension to GEDCOM X.
#[derive(Debug, Deserialize)]
struct ChildAndParents {
    parent1: Option<ResourceReference>,
    parent2: Option<ResourceReference>,
    child: Option<ResourceReference>,
}

/// A parent-child link: `parent` is the father or the mother of `child`.
#[derive(Debug, PartialEq, Eq)]
struct ParentChild {
    parent: PersonId,
    child: PersonId,
}

impl Gedcomx {
    /// Makes a GET and reads the response.
    fn fetch(transport: &Transport, path: &str) -> Result<Self, Error> {
        let response = transport.get(path)?;
        if response.body.is_empty() {
            return Ok(Self::default());
        }
        Ok(serde_json::from_str(&response.body)
            .context("a resposta do FamilySearch não é o GEDCOM X esperado")?)
    }

    /// Reads up to [`BATCH`] persons in one request.
    fn fetch_persons(transport: &Transport, ids: &[PersonId]) -> Result<Self, Error> {
        let pids = ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        Self::fetch(transport, &format!("/platform/tree/persons?pids={pids}"))
    }

    /// The parent-child links in the response. Joins the two forms FamilySearch
    /// sends them in: the GEDCOM X `ParentChild` relationship and the family
    /// (`childAndParentsRelationships`), which is an extension.
    fn parent_child_links(&self) -> Result<Vec<ParentChild>, Error> {
        let standard = self
            .relationships
            .iter()
            .filter(|relationship| {
                relationship.relationship_type == Some(RelationshipType::ParentChild)
            })
            .map(|relationship| (&relationship.person1, &relationship.person2));
        let families = self
            .child_and_parents_relationships
            .iter()
            .flat_map(|family| {
                // Each parent pairs with the child; a family with no child has no link.
                let parents = [&family.parent1, &family.parent2].into_iter().flatten();
                parents.zip(family.child.iter().cycle())
            });
        let mut links = Vec::new();
        for (parent, child) in standard.chain(families) {
            let link = ParentChild {
                parent: referenced_person(parent)?,
                child: referenced_person(child)?,
            };
            if !links.contains(&link) {
                links.push(link);
            }
        }
        Ok(links)
    }
}

fn person_id(person: &Person) -> Option<PersonId> {
    person.id.as_ref()?.to_string().parse().ok()
}

/// The id of the person a reference points to: the end of the URL or fragment.
fn referenced_person(reference: &ResourceReference) -> Result<PersonId, Error> {
    let resource = reference.resource.to_string();
    let id = resource.rsplit(['/', '#']).next().unwrap_or_default();
    Ok(id
        .parse()
        .with_context(|| format!("referência de pessoa inesperada na resposta: {resource}"))?)
}

/// Runs a command of the `fs` group.
///
/// # Errors
/// Any [`Error`], depending on the session and on what FamilySearch answers.
pub fn run(config: &Config, command: &FsCommand) -> Result<(), Error> {
    let transport = Transport::new(config);
    let fetch = |path: String| Gedcomx::fetch(&transport, &path);
    match command {
        FsCommand::Person { id } => {
            for person in fetch(format!("/platform/tree/persons/{id}"))?.persons {
                config.print(&person_line(&person), &person)?;
            }
        }
        FsCommand::Parents { id } => print_relatives(config, &transport, id, Kin::Parents)?,
        FsCommand::Children { id } => print_relatives(config, &transport, id, Kin::Children)?,
        FsCommand::Spouses { id } => print_relatives(config, &transport, id, Kin::Spouses)?,
        FsCommand::Sources { id } => {
            for source in fetch(route(id, "sources"))?.source_descriptions {
                let line = [
                    field(source.id.as_ref()),
                    field(source.titles.first().map(|title| &title.value)),
                    field(source.about.as_ref()),
                ]
                .join("\t");
                config.print(&line, &source)?;
            }
        }
        FsCommand::Notes { id } => {
            for note in notes(fetch(route(id, "notes"))?) {
                let line = [field(note.subject.as_ref()), field(Some(&note.text))].join("\t");
                config.print(&line, &note)?;
            }
        }
        FsCommand::Ancestry {
            traversal,
            cut_year,
        } => traverse(
            config,
            &transport,
            Direction::Ancestry,
            traversal,
            *cut_year,
        )?,
        FsCommand::Descendancy(args) => {
            traverse(config, &transport, Direction::Descendancy, args, None)?;
        }
        FsCommand::Traversal { file } => traversal::list(config, file)?,
        FsCommand::Paths { file, targets } => paths::list(config, file, targets)?,
        FsCommand::Portraits { file, folder } => {
            portraits::download(config, &transport, file, folder)?;
        }
        FsCommand::CreateSource { write, source } => {
            write::create_source(config, &transport, write, source)?;
        }
        FsCommand::AttachSource { write, source } => {
            write::attach_source(config, &transport, write, source)?;
        }
        FsCommand::CreateNote {
            write,
            subject,
            text,
        } => write::create_note(config, &transport, write, subject, text)?,
    }
    Ok(())
}

fn traverse(
    config: &Config,
    transport: &Transport,
    direction: Direction,
    args: &TraversalArgs,
    cut_year: Option<i32>,
) -> Result<(), Error> {
    let TraversalArgs {
        id,
        generations,
        output,
    } = args;
    // A cut year is a limit by itself.
    let generations = generations.or(cut_year.is_none().then_some(GENERATIONS));
    let persons = traversal::traverse(transport, direction, id, generations, cut_year, output)?;
    config.print(
        &format!("{persons}\t{}", output.display()),
        &json!({"pessoas": persons, "arquivo": output}),
    )
}

fn route(id: &PersonId, resource: &str) -> String {
    format!("/platform/tree/persons/{id}/{resource}")
}

/// Which relatives of a person a route lists.
#[derive(Debug, Clone, Copy)]
enum Kin {
    Parents,
    Children,
    Spouses,
}

/// Prints the relatives of `id` that a route lists. The response says who they
/// are through its relationships but may leave their details out (the children
/// route does), so the persons that did not come are read in a batch.
fn print_relatives(
    config: &Config,
    transport: &Transport,
    id: &PersonId,
    kin: Kin,
) -> Result<(), Error> {
    let resource = match kin {
        Kin::Parents => "parents",
        Kin::Children => "children",
        Kin::Spouses => "spouses",
    };
    let gedcomx = Gedcomx::fetch(transport, &route(id, resource))?;
    let links = gedcomx.parent_child_links()?;
    let mut relatives: Vec<PersonId> = match kin {
        Kin::Parents => links
            .into_iter()
            .filter(|link| link.child == *id)
            .map(|link| link.parent)
            .collect(),
        Kin::Children => links
            .into_iter()
            .filter(|link| link.parent == *id)
            .map(|link| link.child)
            .collect(),
        Kin::Spouses => {
            let mut partners = Vec::new();
            for couple in gedcomx.relationships.iter().filter(|relationship| {
                relationship.relationship_type == Some(RelationshipType::Couple)
            }) {
                partners.push(referenced_person(&couple.person1)?);
                partners.push(referenced_person(&couple.person2)?);
            }
            partners
        }
    };
    let mut persons = gedcomx.persons;
    // Whoever came in the response is a relative too, even with no relationship
    // pointing at them (the spouses route sends the other parent of a child).
    relatives.extend(persons.iter().filter_map(person_id));
    let mut seen = HashSet::from([id.clone()]);
    relatives.retain(|relative| seen.insert(relative.clone()));
    let came: HashSet<PersonId> = persons.iter().filter_map(person_id).collect();
    let missing: Vec<PersonId> = relatives
        .iter()
        .filter(|relative| !came.contains(relative))
        .cloned()
        .collect();
    for batch in missing.chunks(BATCH) {
        persons.extend(Gedcomx::fetch_persons(transport, batch)?.persons);
    }
    for relative in &relatives {
        let found = persons
            .iter()
            .find(|person| person_id(person).as_ref() == Some(relative));
        if let Some(person) = found {
            config.print(&person_line(person), person)?;
        }
    }
    Ok(())
}

fn notes(gedcomx: Gedcomx) -> impl Iterator<Item = Note> {
    gedcomx.persons.into_iter().flat_map(|person| person.notes)
}

/// The name the tree shows first for a person.
fn name(person: &Person) -> Option<&str> {
    let form = person.names.first()?.name_forms.first()?;
    form.full_text.as_deref()
}

/// The short line of a person: id, name, sex, birth date and place, death date
/// and place.
fn person_line(person: &Person) -> String {
    // An open enum from another crate: anything but male or female has no sex here.
    let sex = match person.gender.as_ref().map(|gender| &gender.gender_type) {
        Some(GenderType::Male) => Some("M"),
        Some(GenderType::Female) => Some("F"),
        _ => None,
    };
    let fact = |kind: FactType| person.facts.iter().find(move |fact| fact.fact_type == kind);
    let date = |kind| fact(kind)?.date.as_ref()?.original.as_ref();
    let place = |kind| fact(kind)?.place.as_ref()?.original.as_ref();
    [
        field(person.id.as_ref()),
        field(name(person)),
        field(sex),
        field(date(FactType::Birth)),
        field(place(FactType::Birth)),
        field(date(FactType::Death)),
        field(place(FactType::Death)),
    ]
    .join("\t")
}

/// One field of a short line: no TAB or line break, and `-` when empty.
#[must_use = "the field is only useful inside a line"]
pub(crate) fn field(value: Option<impl fmt::Display>) -> String {
    let clean = value
        .map(|value| value.to_string())
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if clean.is_empty() { "-".into() } else { clean }
}
