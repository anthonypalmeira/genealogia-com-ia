//! The full text: what FamilySearch's handwriting recognition read on each
//! page image. Searches it, pulls a whole image group to a file and looks for
//! names in the pulled file.

use std::{
    collections::HashSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Query, ark, read};
use crate::{
    cli::{Config, FullTextArgs},
    coverage::{Coverage, Found, Hit, Kind, Outcome, Search, Seen, count, report},
    error::Error,
    familysearch::field,
    text::{Searchable, unaccent},
    transport::Transport,
};

const ROUTE: &str = "/service/search/fulltext/search";

/// The most page images the route gives in one answer.
const PAGE: u32 = 100;

/// The answer of the route: one entry per page image.
#[derive(Debug, Deserialize)]
struct Results {
    /// How many page images match in all.
    results: u64,
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    /// The image, as `3:1:…`: the end of its ARK.
    id: String,
    collection_title: Option<String>,
    #[serde(default)]
    content: Content,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Content {
    title: Option<String>,
    record_date: Option<String>,
    record_place: Option<String>,
    /// The whole text of the page.
    text_document: String,
    /// The terms that matched, bare.
    highlight_texts: Vec<String>,
}

/// Searches the full text and prints one line per page image.
pub(super) fn search(
    config: &Config,
    transport: &Transport,
    args: &FullTextArgs,
) -> Result<(), Error> {
    let phrases = args.phrases.iter().map(|phrase| required(phrase));
    let text = args
        .text
        .iter()
        .cloned()
        .chain(phrases)
        .collect::<Vec<_>>()
        .join(" ");
    let text = Some(text).filter(|text| !text.is_empty());
    if text.is_none() && args.full_name.is_none() && args.group.is_none() {
        return Err(Error::Input(
            "diga o que buscar: TEXTO, --frase, --nome ou --grupo".into(),
        ));
    }
    for phrase in args.text.iter().flat_map(|text| loose_phrases(text)) {
        eprintln!(
            "garimpo: aviso: a frase \"{phrase}\" está sem `+` (é opcional) ou tem acentos; \
             para obrigá-la use --frase \"{phrase}\""
        );
    }
    let query = Query::new()
        .with("q.text", text.as_deref())
        .with("q.fullName", args.full_name.as_deref())
        .with("q.anyPlace", args.place.as_deref())
        .with("q.anyDate.from", args.from)
        .with("q.anyDate.to", args.to)
        .with("q.groupName", args.group.as_deref())
        .with("count", Some(args.page.count))
        .with("offset", Some(args.page.offset))
        .finish();
    let tagged = |tag: &str, value: Option<&str>| value.map(|value| format!("{tag}:{value}"));
    let search = Search {
        kind: Kind::FullText,
        query: [
            text,
            tagged("nome", args.full_name.as_deref()),
            tagged("grupo", args.group.as_deref()),
        ]
        .into_iter()
        .flatten()
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

/// A phrase as the full text requires it: `+"…"`, without accents.
fn required(phrase: &str) -> String {
    let plain: String = phrase.chars().filter(|c| *c != '"').map(unaccent).collect();
    format!("+\"{}\"", plain.trim())
}

/// The quoted phrases of a query that the full text would not treat as the
/// writer expects: the ones with no `+` or `-` before the opening quote, which
/// are optional, and the ones with accents.
fn loose_phrases(text: &str) -> Vec<&str> {
    let parts: Vec<&str> = text.split('"').collect();
    // Every second part is inside quotes.
    (1..parts.len())
        .step_by(2)
        .filter(|&at| {
            !parts[at - 1].ends_with(['+', '-']) || parts[at].chars().any(|c| unaccent(c) != c)
        })
        .map(|at| parts[at])
        .collect()
}

fn hit(entry: &Entry) -> Hit {
    let Content {
        title,
        record_date,
        record_place,
        text_document,
        highlight_texts,
    } = &entry.content;
    let id = ark(&entry.id);
    let page = Searchable::new(text_document);
    let passage = highlight_texts
        .iter()
        .find_map(|term| page.passage(term))
        .unwrap_or_else(|| page.opening());
    Hit {
        line: [
            field(Some(&id)),
            field(title.as_ref()),
            field(entry.collection_title.as_ref()),
            field(Some(&passage)),
        ]
        .join("\t"),
        json: json!({
            "id": id,
            "titulo": title,
            "data": record_date,
            "lugar": record_place,
            "colecao": entry.collection_title,
            "trecho": passage,
        }),
        id,
    }
}

/// The first line of a pulled group's file: which group it holds.
#[derive(Debug, Serialize, Deserialize)]
struct Header {
    #[serde(rename = "grupo")]
    group: String,
    /// How many page images the group had when the pull started.
    total: u64,
}

/// One page image in a pulled group's file.
#[derive(Debug, Serialize, Deserialize)]
struct Page {
    id: String,
    #[serde(rename = "titulo")]
    title: Option<String>,
    #[serde(rename = "data")]
    date: Option<String>,
    #[serde(rename = "lugar")]
    place: Option<String>,
    #[serde(rename = "texto")]
    text: String,
}

/// A pulled group as its file holds it.
#[derive(Debug)]
struct Pulled {
    header: Header,
    pages: Vec<Page>,
}

impl Pulled {
    /// Reads the file of a pulled group. `None` when there is no file yet.
    fn load(file: &Path) -> Result<Option<Self>, Error> {
        let context = || format!("não consegui usar o arquivo {}", file.display());
        let content = match fs::read_to_string(file) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(anyhow::Error::new(error).context(context()).into()),
        };
        let mut lines = content.lines();
        let not_a_group = || {
            Error::Input(format!(
                "{} não é um arquivo de `garimpo busca grupo`",
                file.display()
            ))
        };
        let header = lines.next().ok_or_else(not_a_group)?;
        let header = serde_json::from_str(header).map_err(|_| not_a_group())?;
        let pages = lines
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .with_context(context)?;
        Ok(Some(Self { header, pages }))
    }

    /// Replaces the file in one step, so an interrupted pull never leaves a
    /// line cut short.
    fn save(&self, file: &Path) -> Result<(), Error> {
        let mut content =
            serde_json::to_string(&self.header).context("não consegui gerar o cabeçalho")?;
        content.push('\n');
        for page in &self.pages {
            content.push_str(&serde_json::to_string(page).context("não consegui gerar a linha")?);
            content.push('\n');
        }
        let draft = draft_path(file);
        fs::write(&draft, content)
            .and_then(|()| fs::rename(&draft, file))
            .with_context(|| format!("não consegui gravar {}", file.display()))?;
        Ok(())
    }

    fn count(&self) -> u64 {
        count(&self.pages)
    }
}

fn draft_path(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".new");
    name.into()
}

/// Pulls the text of every page image of a group into `output`, continuing a
/// file that already exists. Prints how many page images the file holds.
pub(super) fn pull(
    config: &Config,
    transport: &Transport,
    group: &str,
    output: &Path,
    log: &Path,
) -> Result<(), Error> {
    let mut coverage = Coverage::open(log)?;
    let mut pulled = Pulled::load(output)?;
    if let Some(pulled) = &pulled
        && pulled.header.group != group
    {
        return Err(Error::Input(format!(
            "{} é do grupo {}; use outro arquivo em --saida",
            output.display(),
            pulled.header.group
        )));
    }
    let stopped = fill(transport, group, output, &mut pulled);
    let search = Search::new(Kind::Group, group.to_owned());
    let (count, total) = pulled.as_ref().map_or((0, None), |pulled| {
        (pulled.count(), Some(pulled.header.total))
    });
    let seen = Seen {
        total,
        count,
        items: Vec::new(),
        file: Some(output.to_owned()),
    };
    if let Err(error) = stopped {
        coverage.record_failure(search, seen, &error)?;
        return Err(error);
    }
    let total = total.unwrap_or_default();
    coverage.record(search, seen, Outcome::answered(0, count, total))?;
    config.print(
        &format!("{count}\t{total}\t{}", output.display()),
        &json!({"imagens": count, "total": total, "arquivo": output}),
    )
}

/// Asks for the pages of the group that the file does not hold yet and saves
/// each answer as it comes.
fn fill(
    transport: &Transport,
    group: &str,
    output: &Path,
    pulled: &mut Option<Pulled>,
) -> Result<(), Error> {
    let mut seen: HashSet<String> = pulled
        .iter()
        .flat_map(|pulled| &pulled.pages)
        .map(|page| page.id.clone())
        .collect();
    // Before the total the route does not repeat pages, so what the file
    // holds is how far the pull went.
    let mut offset = pulled.as_ref().map_or(0, Pulled::count);
    // Past the total the route repeats pages forever: the pull stops there.
    while pulled
        .as_ref()
        .is_none_or(|pulled| offset < pulled.header.total)
    {
        let query = Query::new()
            .with("q.groupName", Some(group))
            .with("count", Some(PAGE))
            .with("offset", Some(offset))
            .finish();
        let results: Results = read(&transport.get_site(&format!("{ROUTE}?{query}"))?, ROUTE)?;
        let pulled = pulled.get_or_insert_with(|| Pulled {
            header: Header {
                group: group.to_owned(),
                total: results.results,
            },
            pages: Vec::new(),
        });
        let before = pulled.pages.len();
        offset += count(&results.entries);
        for entry in results.entries {
            let id = ark(&entry.id);
            if seen.insert(id.clone()) {
                pulled.pages.push(Page {
                    id,
                    title: entry.content.title,
                    date: entry.content.record_date,
                    place: entry.content.record_place,
                    text: entry.content.text_document,
                });
            }
        }
        pulled.save(output)?;
        // An answer with nothing new would repeat forever too.
        if pulled.pages.len() == before {
            break;
        }
    }
    Ok(())
}

/// Looks for each name in the file of a pulled group and prints one line per
/// page image that holds it.
pub(super) fn names(
    config: &Config,
    file: &Path,
    names: &[String],
    log: &Path,
) -> Result<(), Error> {
    let mut coverage = Coverage::open(log)?;
    let pulled = Pulled::load(file)?
        .ok_or_else(|| Error::Input(format!("não achei o arquivo {}", file.display())))?;
    let complete = pulled.count() >= pulled.header.total;
    if !complete {
        eprintln!(
            "garimpo: aviso: {} tem {} das {} imagens do grupo; nome que não aparecer aqui \
             não é negativo",
            file.display(),
            pulled.count(),
            pulled.header.total
        );
    }
    let pages: Vec<(&Page, Searchable)> = pulled
        .pages
        .iter()
        .map(|page| (page, Searchable::new(&page.text)))
        .collect();
    for name in names {
        let mut found = Vec::new();
        for (page, text) in &pages {
            let Some(passage) = text.passage(name) else {
                continue;
            };
            let line = [
                field(Some(name)),
                field(Some(&page.id)),
                field(page.title.as_ref()),
                field(Some(&passage)),
            ]
            .join("\t");
            config.print(
                &line,
                &json!({"nome": name, "id": page.id, "titulo": page.title, "trecho": passage}),
            )?;
            found.push(page.id.clone());
        }
        if found.is_empty() {
            eprintln!(
                "garimpo: sem ocorrência de \"{name}\" nas {} imagens de {}",
                pages.len(),
                file.display()
            );
        }
        let count = count(&found);
        let outcome = if complete {
            Outcome::answered(0, count, count)
        } else {
            Outcome::Partial
        };
        coverage.record(
            Search::new(
                Kind::Names,
                format!("\"{name}\" no grupo {}", pulled.header.group),
            ),
            Seen {
                total: Some(count),
                count,
                items: found,
                file: Some(file.to_owned()),
            },
            outcome,
        )?;
    }
    Ok(())
}
