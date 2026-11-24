//! The coverage log of a research: every search made, with what it asked and
//! what came of it, so that "was this already searched?" has an answer.
//!
//! One line of JSON per search, in a file of the user's research folder.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    cli::{Config, CoverageCommand},
    error::Error,
    familysearch::field,
    text,
};

/// What came of a search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Outcome {
    /// Everything the search found was seen.
    #[serde(rename = "lido")]
    Read,
    /// There is more than what was seen: the search is still open.
    #[serde(rename = "parcial")]
    Partial,
    /// The search found nothing.
    #[serde(rename = "negativo")]
    Negative,
    /// The search got no answer it could use, which says nothing about what
    /// exists.
    #[serde(rename = "pendente-de-acesso")]
    PendingAccess,
}

impl Outcome {
    /// The outcome of a search that was answered: `seen` items starting at
    /// `offset`, out of `total`.
    #[must_use = "the outcome is only useful when recorded"]
    pub(crate) fn answered(offset: u64, seen: u64, total: u64) -> Self {
        if total == 0 {
            Self::Negative
        } else if offset == 0 && seen >= total {
            Self::Read
        } else {
            Self::Partial
        }
    }
}

/// The name the outcome has in the log.
impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize(f)
    }
}

/// The kind of a search: the command that made it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Kind {
    #[serde(rename = "texto")]
    FullText,
    #[serde(rename = "grupo")]
    Group,
    #[serde(rename = "nomes")]
    Names,
    #[serde(rename = "registros")]
    Records,
    #[serde(rename = "catalogo")]
    Catalog,
    #[serde(rename = "catalogo-item")]
    CatalogItem,
    #[serde(rename = "waypoints")]
    Waypoints,
}

/// The name the kind has in the log.
impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize(f)
    }
}

/// What a search asked.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Search {
    #[serde(rename = "busca")]
    pub(crate) kind: Kind,
    #[serde(rename = "consulta")]
    pub(crate) query: String,
    #[serde(rename = "lugar")]
    pub(crate) place: Option<String>,
    #[serde(rename = "de")]
    pub(crate) from: Option<u16>,
    #[serde(rename = "ate")]
    pub(crate) to: Option<u16>,
    #[serde(rename = "inicio")]
    pub(crate) offset: u64,
}

impl Search {
    /// A search with no place and no years, from its first item.
    #[must_use = "a search is only useful when recorded"]
    pub(crate) fn new(kind: Kind, query: String) -> Self {
        Self {
            kind,
            query,
            place: None,
            from: None,
            to: None,
            offset: 0,
        }
    }
}

/// What a search showed.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Seen {
    /// How many items the search found in all; unknown when it was blocked
    /// before answering.
    pub(crate) total: Option<u64>,
    #[serde(rename = "vistos")]
    pub(crate) count: u64,
    /// The identifiers of the items seen, when they are not in `file`.
    #[serde(rename = "itens")]
    pub(crate) items: Vec<String>,
    /// The file that holds the items seen, or the one that was searched.
    #[serde(rename = "arquivo", skip_serializing_if = "Option::is_none")]
    pub(crate) file: Option<PathBuf>,
}

/// One line of the coverage log.
#[derive(Debug, Serialize, Deserialize)]
struct Record {
    #[serde(rename = "quando")]
    when: String,
    #[serde(flatten)]
    search: Search,
    #[serde(flatten)]
    seen: Seen,
    #[serde(rename = "resultado")]
    outcome: Outcome,
    /// Why access is pending.
    #[serde(rename = "motivo", skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// The coverage log, open for a search to be recorded.
#[derive(Debug)]
pub(crate) struct Coverage {
    file: File,
    path: PathBuf,
}

impl Coverage {
    /// Opens the log before any request, so that no search is made that
    /// cannot be recorded.
    pub(crate) fn open(path: &Path) -> Result<Self, Error> {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .with_context(|| {
                format!(
                    "não consegui abrir o registro de cobertura {}",
                    path.display()
                )
            })?;
        Ok(Self {
            file,
            path: path.to_owned(),
        })
    }

    /// Records a search that was answered.
    pub(crate) fn record(
        &mut self,
        search: Search,
        seen: Seen,
        outcome: Outcome,
    ) -> Result<(), Error> {
        self.append(search, seen, outcome, None)
    }

    /// Records what a failed search leaves behind. A search FamilySearch did
    /// not answer with a result (a 403, a 429, a long pause, an expired login,
    /// a route or an item it did not find, a server or network failure, an
    /// answer in another format) is recorded as pending access, never as a
    /// negative. A search that could not even be asked is recorded only when
    /// part of it was seen before.
    pub(crate) fn record_failure(
        &mut self,
        search: Search,
        seen: Seen,
        error: &Error,
    ) -> Result<(), Error> {
        match error {
            Error::Blocked { .. }
            | Error::Paused { .. }
            | Error::SessionExpired
            // Inside a search, only an answer of FamilySearch is an input
            // error: a redirect or a 404.
            | Error::Input(_)
            | Error::Other(_) => self.append(
                search,
                seen,
                Outcome::PendingAccess,
                Some(format!("{error:#}")),
            ),
            Error::SessionMissing
            | Error::SessionCap { .. }
            | Error::WriteNotAuthorized
            | Error::SourceNotAttached { .. }
            | Error::BridgeDetached
            | Error::InternalRoutesOff => {
                if seen.count == 0 {
                    return Ok(());
                }
                self.append(search, seen, Outcome::Partial, None)
            }
        }
    }

    fn append(
        &mut self,
        search: Search,
        seen: Seen,
        outcome: Outcome,
        reason: Option<String>,
    ) -> Result<(), Error> {
        let record = Record {
            when: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            search,
            seen,
            outcome,
            reason,
        };
        let line =
            serde_json::to_string(&record).context("não consegui gerar a linha da cobertura")?;
        // One write per line: searches that run at the same time append to
        // the same log, and each line has to land whole.
        self.file
            .write_all(format!("{line}\n").as_bytes())
            .with_context(|| {
                format!(
                    "a busca foi feita, mas não consegui registrá-la em {}: {line}",
                    self.path.display()
                )
            })?;
        Ok(())
    }
}

/// Runs a command of the `cobertura` group.
pub(crate) fn run(config: &Config, command: &CoverageCommand) -> Result<(), Error> {
    let CoverageCommand::Query { terms, coverage } = command;
    let path = &coverage.log;
    let context = || {
        format!(
            "não consegui ler o registro de cobertura {}",
            path.display()
        )
    };
    let log = match fs::read_to_string(path) {
        Ok(log) => log,
        // No log yet: nothing was ever searched.
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => return Err(anyhow::Error::new(error).context(context()).into()),
    };
    let mut found = 0;
    for line in log.lines() {
        let record: Record = serde_json::from_str(line).with_context(context)?;
        let Search {
            kind,
            query,
            place,
            from,
            to,
            ..
        } = &record.search;
        let file = record.seen.file.as_ref().map(|file| file.display());
        let searched = format!(
            "{kind} {query} {} {}",
            place.as_deref().unwrap_or_default(),
            file.map(|file| file.to_string()).unwrap_or_default()
        );
        if !terms.iter().all(|term| text::holds(&searched, term)) {
            continue;
        }
        found += 1;
        let year = |year: Option<u16>| year.map(|year| year.to_string()).unwrap_or_default();
        let years = if from.is_some() || to.is_some() {
            Some(format!("{}-{}", year(*from), year(*to)))
        } else {
            None
        };
        let short = [
            record.outcome.to_string(),
            record.when.clone(),
            field(Some(kind)),
            field(Some(query)),
            field(place.as_ref()),
            field(years),
            record.seen.count.to_string(),
            field(record.seen.total),
        ]
        .join("\t");
        config.print(&short, &record)?;
    }
    if found == 0 {
        eprintln!(
            "garimpo: nenhuma busca registrada com esses termos em {}",
            path.display()
        );
    }
    Ok(())
}
