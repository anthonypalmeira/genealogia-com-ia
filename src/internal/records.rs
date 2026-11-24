//! The indexed records: the persons FamilySearch indexed from its images.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Found, Hit, Query, ark, list, read, report, string_at};
use crate::{
    cli::{Config, RecordsArgs},
    coverage::{Kind, Search},
    error::Error,
    familysearch::field,
    transport::Transport,
};

const ROUTE: &str = "/service/search/hr/v2/personas";

/// The answer of the route: one entry per record that matched.
#[derive(Debug, Deserialize)]
struct Results {
    /// How many records match in all.
    results: u64,
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    /// The matched persona: the end of its `1:1:` ARK.
    id: String,
    /// The record as GEDCOM X, under `gedcomx`.
    #[serde(default)]
    content: Value,
}

/// Searches the indexed records and prints one line per record.
pub(super) fn search(
    config: &Config,
    transport: &Transport,
    args: &RecordsArgs,
) -> Result<(), Error> {
    let terms = [
        ("nome", "q.givenName", &args.given_name),
        ("sobrenome", "q.surname", &args.surname),
        ("pai", "q.fatherGivenName", &args.father),
        ("mae", "q.motherGivenName", &args.mother),
        ("conjuge", "q.spouseGivenName", &args.spouse),
        ("colecao", "f.collectionId", &args.collection),
    ];
    if terms.iter().all(|(_, _, value)| value.is_none()) && args.place.is_none() {
        return Err(Error::Input(
            "diga o que buscar: --nome, --sobrenome, --pai, --mae, --conjuge, --lugar ou \
             --colecao"
                .into(),
        ));
    }
    // Without `require`, a term only orders the results.
    let required = |query: Query, parameter: &str, given: bool| {
        let require = (args.require && given).then_some("on");
        query.with(&format!("{parameter}.require"), require)
    };
    let mut query = Query::new();
    for (_, parameter, value) in terms {
        query = query.with(parameter, value.as_deref());
        // The collection is a filter, which already leaves the others out.
        if parameter.starts_with("q.") {
            query = required(query, parameter, value.is_some());
        }
    }
    let query = required(
        query.with("q.anyPlace", args.place.as_deref()),
        "q.anyPlace",
        args.place.is_some(),
    )
    .with("q.birthLikeDate.from", args.from)
    .with("q.birthLikeDate.to", args.to);
    let query = required(
        query,
        "q.birthLikeDate",
        args.from.is_some() || args.to.is_some(),
    )
    .with("count", Some(args.page.count))
    .with("offset", Some(args.page.offset))
    .finish();
    let search = Search {
        kind: Kind::Records,
        query: terms
            .iter()
            .filter_map(|(tag, _, value)| Some(format!("{tag}:{}", value.as_ref()?)))
            .chain(args.require.then(|| "exigir".to_owned()))
            .collect::<Vec<_>>()
            .join(" "),
        place: args.place.clone(),
        from: args.from,
        to: args.to,
        offset: args.page.offset.into(),
    };
    report(config, &args.coverage.log, search, || {
        let results: Results = read(&transport.get_site(&format!("{ROUTE}?{query}"))?, ROUTE)?;
        Ok(Found {
            total: results.results,
            heading: None,
            hits: results.entries.iter().map(hit).collect(),
        })
    })
}

fn hit(entry: &Entry) -> Hit {
    let id = ark(&format!("1:1:{}", entry.id));
    let record = &entry.content["gedcomx"];
    let persons = list(record.get("persons"));
    let name =
        |person: &Value| string_at(person, "/names/0/nameForms/0/fullText").map(str::to_owned);
    // The principal is who the record is about; the others are relatives.
    let principal_at = persons
        .iter()
        .position(|person| person["principal"] == true)
        .unwrap_or(0);
    let principal = persons.get(principal_at);
    let others: Vec<String> = persons
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != principal_at)
        .filter_map(|(_, person)| name(person))
        .collect();
    // The first fact with a date or a place is the event of the record.
    let event = principal.and_then(|person| {
        list(person.get("facts"))
            .iter()
            .find(|fact| fact.get("date").is_some() || fact.get("place").is_some())
    });
    let kind = event
        .and_then(|fact| string_at(fact, "/type"))
        .and_then(|kind| kind.rsplit('/').next());
    let date = event.and_then(|fact| string_at(fact, "/date/original"));
    let place = event.and_then(|fact| string_at(fact, "/place/original"));
    let collection = list(record.get("sourceDescriptions"))
        .iter()
        .find(|description| {
            string_at(description, "/resourceType").is_some_and(|kind| kind.ends_with("Collection"))
        })
        .and_then(|description| string_at(description, "/titles/0/value"));
    let name = principal.and_then(name);
    Hit {
        line: [
            field(Some(&id)),
            field(name.as_ref()),
            field(kind),
            field(date),
            field(place),
            field(collection),
            field(Some(others.join("; "))),
        ]
        .join("\t"),
        json: json!({
            "id": id,
            "nome": name,
            "evento": kind,
            "data": date,
            "lugar": place,
            "colecao": collection,
            "outros": others,
        }),
        id,
    }
}
