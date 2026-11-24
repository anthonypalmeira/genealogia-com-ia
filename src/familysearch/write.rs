//! Writes to the tree: only sources and notes, only in a round the user
//! authorized. Each write first checks whether it already exists, re-reads
//! afterwards and goes into the change log, one writer at a time per account. Relationships, merges and facts are
//! left out.

use std::{
    fs::{File, OpenOptions},
    io::Write,
};

use anyhow::{Context, anyhow};
use chrono::{SecondsFormat, Utc};
use gedcomx::Id;
use serde::Serialize;
use serde_json::json;

use super::{Gedcomx, PersonId, SourceId, notes, route};
use crate::{
    cli::{Config, NewSource, WriteArgs},
    error::Error,
    transport::{Response, Transport},
};

/// What was changed in the tree.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "acao")]
enum Action<'a> {
    #[serde(rename = "criar-fonte")]
    CreateSource {
        #[serde(rename = "fonte")]
        source: &'a SourceId,
    },
    #[serde(rename = "anexar-fonte")]
    AttachSource {
        #[serde(rename = "fonte")]
        source: &'a SourceId,
    },
    #[serde(rename = "criar-nota")]
    CreateNote {
        /// The id FamilySearch gave the note, when it reports one.
        #[serde(rename = "nota", skip_serializing_if = "Option::is_none")]
        note: Option<&'a str>,
    },
}

/// One line of the change log.
#[derive(Debug, Serialize)]
struct Change<'a> {
    #[serde(rename = "quando")]
    when: String,
    #[serde(flatten)]
    action: Action<'a>,
    #[serde(rename = "pessoa")]
    person: &'a PersonId,
    #[serde(rename = "motivo")]
    reason: &'a str,
    #[serde(rename = "rodada")]
    round: &'a str,
    /// `false` on the line written when the server accepts the change, `true`
    /// on the line written once the re-read shows it.
    #[serde(rename = "conferida")]
    verified: bool,
}

/// The change log of an authorized round.
struct ChangeLog<'a> {
    /// Keeps other writers of the same account out from the existence check to
    /// the log line, so two of them cannot both create the same thing.
    _lock: File,
    file: File,
    args: &'a WriteArgs,
    round: &'a str,
}

impl<'a> ChangeLog<'a> {
    /// Refuses a write without an authorized round, and opens the log before
    /// any request, so nothing is written to the tree that cannot be logged.
    fn open(config: &Config, args: &'a WriteArgs) -> Result<Self, Error> {
        let round = args.round.as_deref().map(str::trim).unwrap_or_default();
        if round.is_empty() {
            return Err(Error::WriteNotAuthorized);
        }
        let lock = File::create(config.state_dir()?.join("write.lock"))
            .and_then(|lock| {
                lock.lock()?;
                Ok(lock)
            })
            .with_context(|| {
                format!("não consegui travar a escrita em {}", config.dir.display())
            })?;
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&args.log)
            .with_context(|| format!("não consegui abrir o registro {}", args.log.display()))?;
        Ok(Self {
            _lock: lock,
            file,
            args,
            round,
        })
    }

    fn record(&mut self, action: Action, verified: bool) -> Result<(), Error> {
        let change = Change {
            when: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            action,
            person: &self.args.id,
            reason: &self.args.reason,
            round: self.round,
            verified,
        };
        let line =
            serde_json::to_string(&change).context("não consegui gerar a linha do registro")?;
        writeln!(self.file, "{line}").with_context(|| {
            format!(
                "a alteração foi feita, mas não consegui registrá-la em {}: {line}",
                self.args.log.display()
            )
        })?;
        Ok(())
    }
}

/// The id of what a POST created: the end of the `Location` header URL.
fn created_id(response: &Response) -> Option<&str> {
    let location = response.location.as_deref()?.split('?').next()?;
    location.rsplit('/').next()
}

/// Reads the person's sources and tells whether `source` is among them.
fn has_source(transport: &Transport, person: &PersonId, source: &SourceId) -> Result<bool, Error> {
    let id = Some(Id::from(source.to_string()));
    let sources = Gedcomx::fetch(transport, &route(person, "sources"))?.source_descriptions;
    Ok(sources.iter().any(|description| description.id == id))
}

fn print_already_exists(
    config: &Config,
    id: Option<&SourceId>,
    person: &PersonId,
) -> Result<(), Error> {
    let text = id.map_or_else(|| "-".to_owned(), ToString::to_string);
    config.print(
        &format!("ja-existe\t{text}\t{person}"),
        &json!({"resultado": "ja-existe", "id": id, "pessoa": person}),
    )
}

/// Creates a source and attaches it to the person, unless the person already
/// has a source with the same URL.
pub(super) fn create_source(
    config: &Config,
    transport: &Transport,
    args: &WriteArgs,
    new: &NewSource,
) -> Result<(), Error> {
    let mut log = ChangeLog::open(config, args)?;
    let person = &args.id;
    let existing = Gedcomx::fetch(transport, &route(person, "sources"))?.source_descriptions;
    let same = existing.iter().find(|description| {
        description
            .about
            .as_ref()
            .is_some_and(|url| url.to_string() == new.url)
    });
    if let Some(same) = same {
        let id = same.id.as_ref().and_then(|id| id.to_string().parse().ok());
        return print_already_exists(config, id.as_ref(), person);
    }
    let mut description = json!({
        "about": new.url,
        "titles": [{"value": new.title}],
        "citations": [{"value": new.citation}],
        "attribution": {"changeMessage": args.reason},
    });
    if let Some(note) = &new.note {
        description["notes"] = json!([{"text": note}]);
    }
    let created = transport.post(
        "/platform/sources/descriptions",
        &json!({"sourceDescriptions": [description]}),
    )?;
    let source: SourceId = created_id(&created).and_then(|id| id.parse().ok()).context(
        "o FamilySearch criou a fonte mas não disse o id; ache-a na caixa de fontes antes de repetir",
    )?;
    // The id in the answer is the confirmation that the source exists.
    log.record(Action::CreateSource { source: &source }, true)?;
    // From here on, repeating `criar-fonte` would create another source: the
    // error says how to attach the one that already exists.
    attach(config, transport, &mut log, &source).map_err(|cause| Error::SourceNotAttached {
        source_id: source,
        person: person.clone(),
        cause: Box::new(cause),
    })
}

/// Attaches an existing source to the person, unless it is already attached.
pub(super) fn attach_source(
    config: &Config,
    transport: &Transport,
    args: &WriteArgs,
    source: &SourceId,
) -> Result<(), Error> {
    let mut log = ChangeLog::open(config, args)?;
    if has_source(transport, &args.id, source)? {
        return print_already_exists(config, Some(source), &args.id);
    }
    attach(config, transport, &mut log, source)
}

fn attach(
    config: &Config,
    transport: &Transport,
    log: &mut ChangeLog,
    source: &SourceId,
) -> Result<(), Error> {
    let person = &log.args.id;
    transport.post(
        &format!("/platform/tree/persons/{person}"),
        &json!({"persons": [{"sources": [{
            "description": format!("{}/platform/sources/descriptions/{source}", config.fs_api),
            "attribution": {"changeMessage": log.args.reason},
        }]}]}),
    )?;
    // The server accepted the change: it is logged right away, so an
    // interrupted or failed re-read still leaves a trace. The confirmed line
    // comes only after the re-read shows it.
    log.record(Action::AttachSource { source }, false)?;
    if !has_source(transport, person, source)? {
        return Err(anyhow!(
            "a fonte {source} não apareceu em {person} na releitura; confira no FamilySearch antes de repetir"
        )
        .into());
    }
    log.record(Action::AttachSource { source }, true)?;
    config.print(
        &format!("anexada\t{source}\t{person}"),
        &json!({"resultado": "anexada", "id": source, "pessoa": person}),
    )
}

/// Creates a note on the person, unless the person already has an equal one.
pub(super) fn create_note(
    config: &Config,
    transport: &Transport,
    args: &WriteArgs,
    subject: &str,
    text: &str,
) -> Result<(), Error> {
    let mut log = ChangeLog::open(config, args)?;
    let person = &args.id;
    let has_note = || -> Result<bool, Error> {
        Ok(
            notes(Gedcomx::fetch(transport, &route(person, "notes"))?).any(|note| {
                note.subject.as_deref().map(str::trim) == Some(subject.trim())
                    && note.text.trim() == text.trim()
            }),
        )
    };
    if has_note()? {
        return print_already_exists(config, None, person);
    }
    let created = transport.post(
        &route(person, "notes"),
        &json!({"persons": [{"notes": [{"subject": subject, "text": text}]}]}),
    )?;
    let note = created_id(&created);
    log.record(Action::CreateNote { note }, false)?;
    if !has_note()? {
        return Err(anyhow!(
            "a nota não apareceu em {person} na releitura; confira no FamilySearch antes de repetir"
        )
        .into());
    }
    log.record(Action::CreateNote { note }, true)?;
    config.print(
        &format!("criada\t{}\t{person}", note.unwrap_or("-")),
        &json!({"resultado": "criada", "id": note, "pessoa": person}),
    )
}
