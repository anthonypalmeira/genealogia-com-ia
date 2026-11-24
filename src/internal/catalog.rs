//! The catalog: the library's list of films and books, with the image groups
//! of each item.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Found, Hit, Query, Title, list, read, report, string_at};
use crate::{
    cli::{CatalogArgs, Config},
    coverage::{Kind, Search},
    error::Error,
    familysearch::field,
    transport::Transport,
};

const SEARCH_ROUTE: &str = "/service/search/catalog/v3/search";
const ITEM_ROUTE: &str = "/service/search/catalog/item";

/// The answer of the search route.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Results {
    /// How many items match in all.
    total_hits: u64,
    #[serde(default)]
    search_hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchHit {
    metadata_hit: MetadataHit,
}

#[derive(Debug, Deserialize)]
struct MetadataHit {
    metadata: Metadata,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    /// The URL of the item, which ends in `olib:<id>`.
    identifier: Title,
    #[serde(default)]
    title: Vec<Title>,
    #[serde(default)]
    creator: Vec<String>,
}

/// The answer of the item route.
#[derive(Debug, Deserialize)]
struct Entry {
    source: Source,
}

#[derive(Debug, Deserialize)]
struct Source {
    title: String,
    /// The title with the dates, when the item has them.
    display_title: Option<String>,
    inclusive_dates: Option<String>,
    format: Option<String>,
    /// One note per film: a list, or the note itself when there is only one.
    film_note: Option<Value>,
}

/// Searches the catalog and prints one line per item.
pub(super) fn search(
    config: &Config,
    transport: &Transport,
    args: &CatalogArgs,
) -> Result<(), Error> {
    // The route only searches with its field syntax, each field required.
    let quoted = |name: &str, value: Option<&str>| {
        let value = value?.replace('"', " ");
        Some(format!("+{name}:\"{}\"", value.trim()))
    };
    let query = [
        quoted("place", args.place.as_deref()),
        quoted("title", args.title.as_deref()),
        quoted("keywords", args.keywords.as_deref()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    if query.is_empty() {
        return Err(Error::Input(
            "diga o que buscar: --lugar, --titulo ou --palavras".into(),
        ));
    }
    let path = format!(
        "{SEARCH_ROUTE}?{}",
        Query::new()
            .with("query", Some(&query))
            .with("count", Some(args.page.count))
            .with("offset", Some(args.page.offset))
            .finish()
    );
    let search = Search {
        place: args.place.clone(),
        offset: args.page.offset.into(),
        ..Search::new(Kind::Catalog, query)
    };
    report(config, &args.coverage.log, search, || {
        let results: Results = read(&transport.get_site(&path)?, SEARCH_ROUTE)?;
        Ok(Found {
            total: results.total_hits,
            heading: None,
            hits: results
                .search_hits
                .iter()
                .map(|hit| found_item(&hit.metadata_hit.metadata))
                .collect(),
        })
    })
}

/// One item of a catalog search.
fn found_item(metadata: &Metadata) -> Hit {
    let url = &metadata.identifier.value;
    let id = url.rsplit(['/', ':']).next().unwrap_or(url).to_owned();
    let titles: Vec<&str> = metadata.title.iter().map(|t| t.value.as_str()).collect();
    let title = titles.join(" ");
    Hit {
        line: [
            field(Some(&id)),
            field(Some(&title)),
            field(Some(metadata.creator.join("; "))),
        ]
        .join("\t"),
        json: json!({"id": id, "titulo": title, "autores": metadata.creator}),
        id,
    }
}

/// Reads one catalog item and prints it and its films.
pub(super) fn item(
    config: &Config,
    transport: &Transport,
    id: u64,
    log: &Path,
) -> Result<(), Error> {
    let search = Search::new(Kind::CatalogItem, id.to_string());
    report(config, log, search, || {
        let path = format!("{ITEM_ROUTE}/{id}");
        let Entry { source } = read(&transport.get_site(&path)?, ITEM_ROUTE)?;
        let title = source.display_title.as_ref().unwrap_or(&source.title);
        let hits: Vec<Hit> = list(source.film_note.as_ref()).iter().map(film).collect();
        Ok(Found {
            total: super::count(&hits),
            heading: Some(Hit {
                id: id.to_string(),
                line: [
                    "item".to_owned(),
                    id.to_string(),
                    field(Some(title)),
                    field(source.inclusive_dates.as_ref()),
                    field(source.format.as_ref()),
                ]
                .join("\t"),
                json: json!({
                    "tipo": "item",
                    "id": id,
                    "titulo": title,
                    "datas": source.inclusive_dates,
                    "formato": source.format,
                }),
            }),
            hits,
        })
    })
}

/// One film of a catalog item.
fn film(note: &Value) -> Hit {
    // A number arrives as a number or as text, with or without its zeros.
    let number = |field: &str| match &note[field] {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
        _ => None,
    };
    // An image group is addressed with nine digits: the catalog drops the
    // leading zeros and the image routes need them back.
    let group = number("digital_film_no").map(|number| format!("{number:0>9}"));
    let microfilm = number("filmno");
    let access = match string_at(note, "/digital_film_rights") {
        Some("UNREST") => Some("aberto"),
        Some("NO_ACC") => Some("restrito"),
        other => other,
    };
    let content = [string_at(note, "/text"), string_at(note, "/items")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
    Hit {
        id: group
            .clone()
            .or_else(|| microfilm.clone())
            .unwrap_or_default(),
        line: [
            "filme".to_owned(),
            field(group.as_ref()),
            field(microfilm.as_ref()),
            field(access),
            field(Some(&content)),
        ]
        .join("\t"),
        json: json!({
            "tipo": "filme",
            "grupo": group,
            "microfilme": microfilm,
            "acesso": access,
            "conteudo": content,
        }),
    }
}
