//! digitarq: the catalog of the Portuguese archives (Torre do Tombo and the
//! district archives), through its public API and its OAI-PMH endpoint.
//!
//! It takes no session and has nothing to do with the FamilySearch account:
//! requests go a third of a second apart, and a block ends the command.
//! Every query goes into the coverage log.

use std::{
    cell::Cell,
    fs::{self, File},
    io::{self, ErrorKind},
    path::Path,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use ureq::{Agent, Body, http::Response};

use crate::{
    cli::{Config, DigitarqCommand, DocumentId, RangeArgs},
    coverage::{Coverage, Found, Hit, Kind, Outcome, Search, Seen, count, report},
    error::{Error, Service},
    familysearch::field,
    images::{
        Tally, inspect,
        manifest::{Line, Manifest},
        numbers, part_path, settled,
    },
    transport::agent,
};

/// The wait between two requests, which the archive's server takes well.
const PAUSE: Duration = Duration::from_millis(300);

/// How many files of a document are asked for in one request.
const FILES_PAGE: usize = 2000;

/// Requests to digitarq, spaced apart.
struct Client<'a> {
    config: &'a Config,
    agent: Agent,
    /// When the last request was answered.
    last: Cell<Option<Instant>>,
}

impl<'a> Client<'a> {
    fn new(config: &'a Config) -> Self {
        Self {
            config,
            agent: agent(),
            last: Cell::new(None),
        }
    }

    /// Makes a GET and returns the answer when it is a success. A 403 or a
    /// 429 is a block: it is not tried again.
    fn get(&self, path: &str) -> Result<Response<Body>, Error> {
        if let Some(last) = self.last.get() {
            thread::sleep(PAUSE.saturating_sub(last.elapsed()));
        }
        let answered = self
            .agent
            .get(&format!("{}{path}", self.config.digitarq))
            .call();
        self.last.set(Some(Instant::now()));
        let response = answered
            .map_err(|error| anyhow!(error).context(format!("o pedido GET {path} falhou")))?;
        match response.status().as_u16() {
            200..=299 => Ok(response),
            status @ (403 | 429) => Err(Error::Blocked {
                service: Service::Digitarq,
                status,
                path: path.to_owned(),
            }),
            404 => Err(Error::Input(format!("o digitarq não achou {path} (404)"))),
            status => Err(anyhow!("o digitarq respondeu {status} em GET {path}").into()),
        }
    }

    fn text(&self, path: &str) -> Result<String, Error> {
        Ok(self
            .get(path)?
            .body_mut()
            .read_to_string()
            .with_context(|| format!("não consegui ler a resposta de GET {path}"))?)
    }

    fn json<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        Ok(serde_json::from_str(&self.text(path)?).with_context(|| {
            format!("a resposta de {path} não veio no formato esperado; a rota pode ter mudado")
        })?)
    }
}

/// Runs a command of the `digitarq` group.
pub(crate) fn run(config: &Config, command: &DigitarqCommand) -> Result<(), Error> {
    let client = Client::new(config);
    match command {
        DigitarqCommand::Search {
            text,
            phrase,
            page,
            coverage,
        } => {
            let query = form_urlencoded::Serializer::new(String::new())
                .append_pair("query", text)
                .append_pair(
                    "searchType",
                    if *phrase { "ALL_PHRASE" } else { "ALL_WORD" },
                )
                .append_pair("fromIndex", &page.offset.to_string())
                .append_pair("max", &page.count.to_string())
                .finish();
            let search = Search {
                offset: page.offset.into(),
                ..Search::new(Kind::DigitarqSearch, text.clone())
            };
            report(config, &coverage.log, search, || {
                let found: Results<Document> = client.json(&format!("/api/docs/search?{query}"))?;
                Ok(Found {
                    total: found.total,
                    heading: None,
                    hits: found.results.iter().map(document_hit).collect(),
                })
            })
        }
        DigitarqCommand::Document { id, coverage } => {
            let search = Search::new(Kind::DigitarqDocument, id.to_string());
            report(config, &coverage.log, search, || {
                let fields: Map<String, Value> = client.json(&format!("/api/docs/frozen/{id}"))?;
                let hits: Vec<Hit> = fields
                    .iter()
                    .filter_map(|(name, value)| Some((name, value.as_str()?.trim())))
                    .filter(|(_, value)| !value.is_empty())
                    .map(|(name, value)| Hit {
                        id: name.clone(),
                        line: [name.clone(), field(Some(value))].join("\t"),
                        json: json!({"campo": name, "valor": value}),
                    })
                    .collect();
                Ok(Found {
                    total: count(&hits),
                    heading: None,
                    hits,
                })
            })
        }
        DigitarqCommand::Files { id, coverage } => {
            let search = Search::new(Kind::DigitarqFiles, id.to_string());
            report(config, &coverage.log, search, || {
                let files = files(&client, id)?;
                Ok(Found {
                    total: count(&files),
                    heading: None,
                    hits: files.iter().zip(1u32..).map(file_hit).collect(),
                })
            })
        }
        DigitarqCommand::Download {
            id,
            representation,
            range,
            folder,
            coverage,
        } => download(
            config,
            &client,
            id,
            representation.as_deref(),
            range,
            &folder.folder,
            &coverage.log,
        ),
        DigitarqCommand::Oai {
            set,
            output,
            from,
            until,
            pages,
            coverage,
        } => {
            let stamp = |tag: &str, date: Option<chrono::NaiveDate>| {
                date.map(|date| format!(" {tag} {date}"))
                    .unwrap_or_default()
            };
            let harvest = Harvest {
                label: format!(
                    "digitarq:{set}{}{}",
                    stamp("desde", *from),
                    stamp("ate", *until)
                ),
                first: form_urlencoded::Serializer::new(String::new())
                    .append_pair("verb", "ListRecords")
                    .append_pair("metadataPrefix", "oai_dc")
                    .append_pair("set", &set.to_string())
                    .extend_pairs(from.map(|date| ("from", date.to_string())))
                    .extend_pairs(until.map(|date| ("until", date.to_string())))
                    .finish(),
                output,
            };
            harvest.run(config, &client, *pages, &coverage.log)
        }
    }
}

/// A page of results of the API.
#[derive(Debug, Deserialize)]
struct Results<T> {
    results: Vec<T>,
    /// How many there are in all.
    total: u64,
}

/// A value of a description, which the API wraps in an object.
#[derive(Debug, Deserialize)]
struct Text {
    value: Option<String>,
}

/// A description found by the search.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    id: String,
    reference_code: Option<Text>,
    date_initial: Option<Text>,
    date_final: Option<Text>,
    #[serde(default)]
    titles: Vec<Text>,
    /// How many images the document has.
    #[serde(default)]
    files_count: u64,
}

fn document_hit(document: &Document) -> Hit {
    let value = |text: Option<&Text>| text.and_then(|text| text.value.clone());
    let code = value(document.reference_code.as_ref());
    let dates = [&document.date_initial, &document.date_final]
        .map(Option::as_ref)
        .map(value)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" a ");
    let title = document.titles.iter().find_map(|title| title.value.clone());
    Hit {
        id: document.id.clone(),
        line: [
            field(Some(&document.id)),
            field(code.as_ref()),
            field(Some(&dates)),
            field(title.as_ref()),
            document.files_count.to_string(),
        ]
        .join("\t"),
        json: json!({
            "id": document.id, "cota": code, "datas": dates, "titulo": title,
            "imagens": document.files_count,
        }),
    }
}

/// One image of a document.
#[derive(Debug, Deserialize)]
struct DocumentFile {
    id: String,
    /// The file name the archive gives the image.
    name: String,
    /// Which of the document's digitizations the image belongs to.
    #[serde(rename = "representationID")]
    representation: Option<String>,
}

/// Every image of a document, in order.
fn files(client: &Client, id: &DocumentId) -> Result<Vec<DocumentFile>, Error> {
    let mut files = Vec::new();
    loop {
        let page: Results<DocumentFile> = client.json(&format!(
            "/api/rdigital/{id}?fromIndex={}&max={FILES_PAGE}",
            files.len()
        ))?;
        let came = page.results.len();
        files.extend(page.results);
        // An answer with nothing would repeat forever.
        if came == 0 || count(&files) >= page.total {
            return Ok(files);
        }
    }
}

fn file_hit((file, number): (&DocumentFile, u32)) -> Hit {
    Hit {
        id: file.id.clone(),
        line: [
            number.to_string(),
            field(Some(&file.id)),
            field(Some(&file.name)),
            field(file.representation.as_ref()),
        ]
        .join("\t"),
        json: json!({
            "numero": number, "id": file.id, "nome": file.name,
            "representacao": file.representation,
        }),
    }
}

/// Downloads the images of a document into the folder, one at a time,
/// recording each one in the manifest and the whole in the coverage log.
fn download(
    config: &Config,
    client: &Client,
    id: &DocumentId,
    representation: Option<&str>,
    range: &RangeArgs,
    folder: &Path,
    log: &Path,
) -> Result<(), Error> {
    let mut coverage = Coverage::open(log)?;
    let mut manifest = Manifest::open(folder)?;
    let group = format!("digitarq:{id}");
    let mut tally = Tally::default();
    let mut total = None;
    let fetched = (|| {
        let files = files(client, id)?;
        total = Some(count(&files));
        manifest.record(Line::Listed {
            group: group.clone(),
            total: u32::try_from(files.len()).context("o documento tem imagens demais")?,
        })?;
        if files.is_empty() {
            return Ok(());
        }
        let wanted = numbers(range, u32::try_from(files.len()).unwrap_or(u32::MAX))?;
        for (file, number) in files.iter().zip(1u32..) {
            let other =
                representation.is_some_and(|only| file.representation.as_deref() != Some(only));
            if !wanted.contains(&number) || other {
                continue;
            }
            // A line of another file is not this image's: the list changed.
            let line = manifest.image(&group, number).filter(|line| match line {
                Line::Downloaded { id, .. } | Line::Pending { id, .. } => *id == file.id,
                Line::Listed { .. } => false,
            });
            if let Some(settled) = settled(folder, line, false) {
                tally.settle(settled);
                continue;
            }
            let line = fetch(client, folder, &group, number, file)?;
            match &line {
                Line::Downloaded { .. } => tally.new += 1,
                Line::Pending { reason, .. } => {
                    tally.pending += 1;
                    eprintln!("garimpo: imagem {number} pendente de acesso: {reason}");
                }
                Line::Listed { .. } => {}
            }
            manifest.record(line)?;
        }
        Ok(())
    })();
    manifest.compact()?;
    let search = Search::new(Kind::DigitarqDownload, id.to_string());
    let seen = Seen {
        total,
        count: u64::from(tally.new + tally.kept),
        items: Vec::new(),
        file: Some(folder.to_owned()),
    };
    if let Err(error) = fetched {
        coverage.record_failure(search, seen, &error)?;
        return Err(error);
    }
    let outcome = Outcome::answered(0, seen.count, total.unwrap_or_default());
    coverage.record(search, seen, outcome)?;
    tally.print(config, &group, folder)
}

/// Downloads one image of a document. The file takes the name the archive
/// gives it, the id of the file and the extension of what it really holds.
fn fetch(
    client: &Client,
    folder: &Path,
    group: &str,
    number: u32,
    file: &DocumentFile,
) -> Result<Line, Error> {
    // The id and the name come from the server and go into an address and
    // into a path: the id as it is, and only the last part of the name.
    if !file.id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(anyhow!("o digitarq deu um id de arquivo inesperado: {}", file.id).into());
    }
    let stem = Path::new(&file.name).file_stem().map_or_else(
        || file.id.clone(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let part = part_path(&folder.join(&stem));
    let route = format!("/api/rdigital/dissemination?fileId={}", file.id);
    let stored = client.get(&route).and_then(|mut response| {
        File::create(&part)
            .and_then(|mut into| io::copy(&mut response.body_mut().as_reader(), &mut into))
            .with_context(|| format!("não consegui gravar {}", part.display()))?;
        inspect(&part)
    });
    let line = match stored {
        Ok(Some(image)) => {
            // The name the archive gives says `.jpg` even for a TIFF.
            let extension = image.format.extensions_str().first().unwrap_or(&"bin");
            // Two digitizations of one document repeat the names: the id
            // of the file keeps them apart.
            let name = format!("{stem}.{}.{extension}", file.id);
            fs::rename(&part, folder.join(&name))
                .with_context(|| format!("não consegui gravar {name}"))?;
            return Ok(Line::Downloaded {
                group: group.to_owned(),
                number,
                id: file.id.clone(),
                file: name,
                sha256: image.sha256,
                bytes: image.bytes,
                width: image.width,
                height: image.height,
            });
        }
        Ok(None) => Ok(Line::Pending {
            group: group.to_owned(),
            number,
            id: file.id.clone(),
            reason: "o que veio não é uma imagem".to_owned(),
        }),
        Err(error) => Err(error),
    };
    // What a download that did not complete left behind, if anything.
    let _ = fs::remove_file(&part);
    line
}

/// An answer of the OAI-PMH endpoint.
#[derive(Debug, Deserialize)]
struct OaiPmh {
    #[serde(rename = "ListRecords")]
    list: Option<ListRecords>,
    error: Option<OaiError>,
}

#[derive(Debug, Deserialize)]
struct OaiError {
    #[serde(rename = "@code")]
    code: String,
    #[serde(rename = "$text", default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct ListRecords {
    #[serde(default)]
    record: Vec<Record>,
    #[serde(rename = "resumptionToken")]
    token: Option<Token>,
}

/// Where the next page starts; empty on the last page.
#[derive(Debug, Deserialize)]
struct Token {
    /// How many records the whole list has.
    #[serde(rename = "@completeListSize")]
    size: Option<u64>,
    #[serde(rename = "$text", default)]
    value: String,
}

#[derive(Debug, Deserialize)]
struct Record {
    header: RecordHeader,
    metadata: Option<Metadata>,
}

#[derive(Debug, Deserialize)]
struct RecordHeader {
    identifier: String,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    dc: DublinCore,
}

/// A description in Dublin Core, as the endpoint gives it.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct DublinCore {
    title: Vec<String>,
    description: Vec<String>,
    date: Vec<String>,
    /// The reference code and the address of the description.
    identifier: Vec<String>,
}

/// One harvest of descriptions into a file. The file has the format of
/// `garimpo busca grupo`, so `garimpo busca nomes` searches it; its first
/// line also says where a harvest that is not over continues from.
struct Harvest<'a> {
    /// What identifies the harvest: the set and the dates.
    label: String,
    /// The query of the first page.
    first: String,
    output: &'a Path,
}

impl Harvest<'_> {
    /// Harvests up to `pages` pages, continuing a file that already exists,
    /// and writes the file once, in one step, with what this round brought.
    fn run(&self, config: &Config, client: &Client, pages: u32, log: &Path) -> Result<(), Error> {
        let mut coverage = Coverage::open(log)?;
        let output = self.output;
        let context = || format!("não consegui usar o arquivo {}", output.display());
        let held = match fs::read_to_string(output) {
            Ok(held) => Some(held),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => return Err(anyhow::Error::new(error).context(context()).into()),
        };
        // The first line says which harvest the file holds and where it
        // stopped; the rest are the descriptions.
        let (header, mut lines) = match &held {
            Some(held) => {
                let (header, lines) = held.split_once('\n').unwrap_or((held, ""));
                let header: Value = serde_json::from_str(header).with_context(context)?;
                (Some(header), lines.to_owned())
            }
            None => (None, String::new()),
        };
        if let Some(header) = &header
            && header["grupo"] != self.label.as_str()
        {
            return Err(Error::Input(format!(
                "{} é de outra colheita ({}); use outro arquivo em --saida",
                output.display(),
                header["grupo"]
            )));
        }
        let mut seen = count(&lines.lines().collect::<Vec<_>>());
        let mut token = header
            .as_ref()
            .and_then(|header| header["continuacao"].as_str())
            .map(str::to_owned);
        // A file that tells no place to continue from is a harvest that ended.
        let mut over = header.is_some() && token.is_none();
        // How many descriptions the server says the list has: an estimate.
        let mut told = header.as_ref().and_then(|header| header["total"].as_u64());
        let mut harvest = || -> Result<(), Error> {
            for _ in 0..pages {
                if over {
                    break;
                }
                let query = match &token {
                    Some(token) => form_urlencoded::Serializer::new(String::new())
                        .append_pair("verb", "ListRecords")
                        .append_pair("resumptionToken", token)
                        .finish(),
                    None => self.first.clone(),
                };
                let page = list_records(client, &query)?;
                for record in &page.record {
                    lines.push_str(&description(record).to_string());
                    lines.push('\n');
                }
                seen += count(&page.record);
                told = page.token.as_ref().and_then(|next| next.size).or(told);
                // An empty token, or none, ends the list.
                token = page
                    .token
                    .map(|next| next.value.trim().to_owned())
                    .filter(|next| !next.is_empty());
                over = token.is_none();
            }
            Ok(())
        };
        let harvested = harvest();
        // A harvest that is over has what it holds. One that goes on has at
        // least one description more, whatever the server estimated: a name
        // not found in it yet is not a negative.
        let total = if over {
            seen
        } else {
            told.unwrap_or_default().max(seen + 1)
        };
        if over || seen > 0 {
            // Where to continue from goes in the same file as the pages, so
            // one cannot be saved without the other.
            let header = json!({"grupo": self.label, "total": total, "continuacao": token});
            let draft = part_path(output);
            fs::write(&draft, format!("{header}\n{lines}"))
                .and_then(|()| fs::rename(&draft, output))
                .with_context(context)?;
        }
        let search = Search::new(Kind::DigitarqOai, self.label.clone());
        let shown = Seen {
            total: Some(total),
            count: seen,
            items: Vec::new(),
            file: Some(output.to_owned()),
        };
        if let Err(error) = harvested {
            coverage.record_failure(search, shown, &error)?;
            return Err(error);
        }
        let outcome = if over {
            Outcome::answered(0, seen, seen)
        } else {
            eprintln!(
                "garimpo: {seen} de {total} descrições; repita o mesmo comando para continuar"
            );
            Outcome::Partial
        };
        coverage.record(search, shown, outcome)?;
        config.print(
            &format!("{seen}\t{total}\t{}", output.display()),
            &json!({"descricoes": seen, "total": total, "arquivo": output}),
        )
    }
}

/// Asks for one page of records. A list with no records is an empty page.
fn list_records(client: &Client, query: &str) -> Result<ListRecords, Error> {
    let answer: OaiPmh = quick_xml::de::from_str(&client.text(&format!("/oai?{query}"))?)
        .context("a resposta do OAI-PMH não veio no formato esperado")?;
    match (answer.list, answer.error) {
        (Some(list), _) => Ok(list),
        (None, Some(error)) if error.code == "noRecordsMatch" => Ok(ListRecords {
            record: Vec::new(),
            token: None,
        }),
        (None, Some(error)) if error.code == "badResumptionToken" => Err(Error::Input(format!(
            "o digitarq não aceita mais a continuação desta colheita ({}): apague o arquivo e \
             recomece",
            error.message.trim()
        ))),
        (None, Some(error)) => Err(anyhow!(
            "o digitarq recusou o pedido OAI-PMH: {}: {}",
            error.code,
            error.message.trim()
        )
        .into()),
        (None, None) => Err(anyhow!("a resposta do OAI-PMH não trouxe registros nem erro").into()),
    }
}

/// One harvested description, as a line of the file: the title and the
/// descriptions make the text that `garimpo busca nomes` searches.
fn description(record: &Record) -> Value {
    let dc = record.metadata.as_ref().map(|metadata| &metadata.dc);
    let of = |values: fn(&DublinCore) -> &Vec<String>| dc.map(values).cloned().unwrap_or_default();
    let (titles, descriptions) = (of(|dc| &dc.title), of(|dc| &dc.description));
    let code = of(|dc| &dc.identifier)
        .into_iter()
        .find(|identifier| !identifier.starts_with("http"));
    let text = [titles.as_slice(), descriptions.as_slice()]
        .concat()
        .join("\n");
    let dates = Some(of(|dc| &dc.date).join("/")).filter(|dates| !dates.is_empty());
    json!({
        "id": code.unwrap_or_else(|| record.header.identifier.clone()),
        "titulo": titles.first(),
        "data": dates,
        "lugar": null,
        "texto": text,
    })
}
