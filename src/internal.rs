//! The internal FamilySearch routes: the ones the website itself uses and
//! FamilySearch does not document for third parties. They are the only way to
//! search the full text, the indexed records and the catalog.
//!
//! They change without notice, so every one of them lives in this module,
//! which `--rotas-internas desligadas` switches off as a whole. Each search
//! goes into the coverage log by itself, including the ones that find nothing
//! and the ones that are blocked.

mod catalog;
mod fulltext;
mod records;
mod waypoints;

use std::{fmt, str::FromStr};

use anyhow::Context;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    cli::{Config, InternalRoutes, SearchCommand},
    error::Error,
    transport::{Response, Transport},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("identificador inválido: use só letras, números, hífen, dois-pontos e vírgula")]
pub struct InvalidRouteId;

/// The identifier of a collection or of a waypoint, such as `9990001` or
/// `SINT-V1:100,101`: only what can go into a URL as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteId(String);

impl FromStr for RouteId {
    type Err = InvalidRouteId;

    fn from_str(text: &str) -> Result<Self, InvalidRouteId> {
        let valid = !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | ':' | ','));
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(InvalidRouteId)
        }
    }
}

impl fmt::Display for RouteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Runs a command of the `busca` group.
///
/// # Errors
/// `Error::InternalRoutesOff` when the internal routes are switched off, and
/// otherwise any [`Error`], depending on the session and on what FamilySearch
/// answers.
pub fn run(config: &Config, command: &SearchCommand) -> Result<(), Error> {
    let online = || match config.internal_routes {
        InternalRoutes::On => Ok(Transport::new(config)),
        InternalRoutes::Off => Err(Error::InternalRoutesOff),
    };
    match command {
        SearchCommand::FullText(args) => fulltext::search(config, &online()?, args),
        SearchCommand::Group {
            group,
            output,
            coverage,
        } => fulltext::pull(config, &online()?, group, output, &coverage.log),
        // A search in a file already pulled: no route involved.
        SearchCommand::Names {
            file,
            names,
            coverage,
        } => fulltext::names(config, file, names, &coverage.log),
        SearchCommand::Records(args) => records::search(config, &online()?, args),
        SearchCommand::Catalog(args) => catalog::search(config, &online()?, args),
        SearchCommand::CatalogItem { id, coverage } => {
            catalog::item(config, &online()?, *id, &coverage.log)
        }
        SearchCommand::Waypoints {
            collection,
            waypoint,
            count,
            offset,
            coverage,
        } => waypoints::list_level(
            config,
            &online()?,
            collection,
            waypoint.as_ref(),
            *count,
            *offset,
            &coverage.log,
        ),
    }
}

/// A URL query being built: only the parameters that have a value go in.
struct Query(form_urlencoded::Serializer<'static, String>);

impl Query {
    fn new() -> Self {
        Self(form_urlencoded::Serializer::new(String::new()))
    }

    fn with(mut self, name: &str, value: Option<impl fmt::Display>) -> Self {
        if let Some(value) = value {
            self.0.append_pair(name, &value.to_string());
        }
        self
    }

    fn finish(mut self) -> String {
        self.0.finish()
    }
}

/// Reads the JSON of a response.
fn read<T: DeserializeOwned>(response: &Response, route: &str) -> Result<T, Error> {
    Ok(serde_json::from_str(&response.body).with_context(|| {
        format!("a resposta de {route} não veio no formato esperado; a rota pode ter mudado")
    })?)
}

/// The text at `pointer` (a JSON pointer) inside `value`.
fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer)?.as_str()
}

/// The items of a JSON field that FamilySearch sends as a list, as one bare
/// value when there is only one, or not at all.
fn list(value: Option<&Value>) -> &[Value] {
    match value {
        None | Some(Value::Null) => &[],
        Some(Value::Array(items)) => items,
        Some(single) => std::slice::from_ref(single),
    }
}

/// A title, as GEDCOM X and the catalog carry it.
#[derive(Debug, Deserialize)]
struct Title {
    value: String,
}

/// The ARK of a record, a persona or an image, from the end FamilySearch
/// sends, such as `3:1:…`.
fn ark(id: &str) -> String {
    format!("ark:/61903/{id}")
}
