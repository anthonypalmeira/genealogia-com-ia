//! The page images of a FamilySearch film: how many there are, the thumbnails
//! for triage and the full images, kept in a folder with a manifest. The
//! images are addressed by film and number, with no index to ask.

mod edit;
pub(crate) mod manifest;

use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Mutex, mpsc},
    thread,
};

use anyhow::Context;
use image::{ImageFormat, ImageReader};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use self::manifest::{Line, Manifest};
use crate::{
    cli::{Config, Film, ImagesCommand, RangeArgs},
    error::Error,
    familysearch::field,
    transport::Transport,
};

/// How many downloads go by between two lines of progress.
const PROGRESS_EVERY: u32 = 10;

/// Runs a command of the `imagens` group.
pub(crate) fn run(config: &Config, command: &ImagesCommand) -> Result<(), Error> {
    match command {
        ImagesCommand::List { film, folder } => list(config, film, &folder.folder),
        ImagesCommand::Download {
            film,
            range,
            folder,
            workers,
            retry_pending,
        } => download(
            config,
            film,
            range,
            &folder.folder,
            usize::from(*workers),
            *retry_pending,
        ),
        ImagesCommand::Thumbnails {
            film,
            range,
            folder,
        } => thumbnails(config, film, range, &folder.folder),
        ImagesCommand::Crop {
            file,
            region,
            enlarge,
        } => edit::crop(config, file, *region, *enlarge),
        ImagesCommand::Contrast { file } => edit::contrast(config, file),
        ImagesCommand::Sheet {
            files,
            output,
            columns,
        } => edit::sheet(config, files, output, *columns),
    }
}

/// What the storage tells about an image group.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Group {
    /// How many images the group has.
    child_count: u32,
}

/// The address of one image of a film in the storage.
fn node(film: &Film, number: u32) -> String {
    format!("dgs:{film}_{number:05}")
}

/// The stable name of one image of a film: the film and the image number.
fn file_name(film: &Film, number: u32) -> String {
    format!("{film}_{number:05}.jpg")
}

/// How many images the film has: what the manifest already knows, or what
/// the storage answers, which is then recorded.
fn total(transport: &Transport, manifest: &mut Manifest, film: &Film) -> Result<u32, Error> {
    if let Some(total) = manifest.total(&film.to_string()) {
        return Ok(total);
    }
    ask_total(transport, manifest, film)
}

/// Asks the storage how many images the film has and records the answer.
fn ask_total(transport: &Transport, manifest: &mut Manifest, film: &Film) -> Result<u32, Error> {
    let route = format!("/dgs:{film}");
    let group: Group = serde_json::from_str(&transport.get_storage(&route)?.body)
        .with_context(|| format!("a resposta de {route} não veio no formato esperado"))?;
    manifest.record(Line::Listed {
        group: film.to_string(),
        total: group.child_count,
    })?;
    Ok(group.child_count)
}

/// The numbers a command covers, out of the `total` images that exist.
pub(crate) fn numbers(
    range: &RangeArgs,
    total: u32,
) -> Result<std::ops::RangeInclusive<u32>, Error> {
    let last = range.to.unwrap_or(total).min(total);
    if range.from > last {
        return Err(Error::Input(format!(
            "não há imagens de {} a {}: são {total} ao todo",
            range.from,
            range.to.unwrap_or(total)
        )));
    }
    Ok(range.from..=last)
}

/// Numbers in order as short ranges, such as `1-40,45`.
fn ranges(numbers: impl IntoIterator<Item = u32>) -> Option<String> {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for number in numbers {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == number => *last = number,
            _ => runs.push((number, number)),
        }
    }
    let text = runs
        .iter()
        .map(|(first, last)| {
            if first == last {
                first.to_string()
            } else {
                format!("{first}-{last}")
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    Some(text).filter(|text| !text.is_empty())
}

/// Tells how many images the film has and which ones the folder holds.
fn list(config: &Config, film: &Film, folder: &Path) -> Result<(), Error> {
    let mut manifest = Manifest::open(folder)?;
    let total = ask_total(&Transport::new(config), &mut manifest, film)?;
    manifest.compact()?;
    let group = film.to_string();
    let downloaded = ranges(manifest.images(&group).filter_map(|line| match line {
        Line::Downloaded { number, .. } => Some(*number),
        Line::Listed { .. } | Line::Pending { .. } => None,
    }));
    let pending = ranges(manifest.images(&group).filter_map(|line| match line {
        Line::Pending { number, .. } => Some(*number),
        Line::Listed { .. } | Line::Downloaded { .. } => None,
    }));
    config.print(
        &[
            group.clone(),
            total.to_string(),
            field(downloaded.as_ref()),
            field(pending.as_ref()),
        ]
        .join("\t"),
        &json!({"grupo": group, "total": total, "baixadas": downloaded, "pendentes": pending}),
    )
}

/// What a file in the folder is, once checked to be an image.
#[derive(Debug)]
pub(crate) struct Inspected {
    pub(crate) sha256: String,
    pub(crate) bytes: u64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: ImageFormat,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Reads a file and tells its hash and its size in pixels. `None` when it
/// does not hold a whole image: the image is decoded to its end.
pub(crate) fn inspect(file: &Path) -> Result<Option<Inspected>, Error> {
    let context = || format!("não consegui ler {}", file.display());
    let bytes = fs::read(file).with_context(context)?;
    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .with_context(context)?;
    let Some(format) = reader.format() else {
        return Ok(None);
    };
    let Ok(image) = reader.decode() else {
        return Ok(None);
    };
    // The JPEG decoder fills in what a file cut short lacks: a whole JPEG
    // has its end marker.
    if format == ImageFormat::Jpeg && !bytes.windows(2).any(|pair| pair == [0xFF, 0xD9]) {
        return Ok(None);
    }
    Ok(Some(Inspected {
        sha256: sha256(&bytes),
        bytes: u64::try_from(bytes.len()).expect("a length fits in 64 bits"),
        width: image.width(),
        height: image.height(),
        format,
    }))
}

/// Whether the manifest already answers for an image: it is in the folder
/// with the hash the manifest holds, or its access is pending and is not to
/// be tried again.
#[must_use = "the answer decides whether the image is asked for"]
pub(crate) fn settled(folder: &Path, line: Option<&Line>, retry_pending: bool) -> Option<Settled> {
    match line? {
        Line::Downloaded {
            file, sha256: held, ..
        } => fs::read(folder.join(file))
            .is_ok_and(|bytes| sha256(&bytes) == *held)
            .then_some(Settled::Kept),
        Line::Pending { .. } => (!retry_pending).then_some(Settled::Pending),
        Line::Listed { .. } => None,
    }
}

/// How an image that needs no request stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Settled {
    Kept,
    Pending,
}

/// How the images of a command ended up.
#[derive(Debug, Default)]
pub(crate) struct Tally {
    pub(crate) new: u32,
    pub(crate) kept: u32,
    pub(crate) pending: u32,
}

impl Tally {
    pub(crate) fn settle(&mut self, settled: Settled) {
        match settled {
            Settled::Kept => self.kept += 1,
            Settled::Pending => self.pending += 1,
        }
    }

    /// Prints the line of a download: group, new, already there, pending
    /// access, folder.
    pub(crate) fn print(&self, config: &Config, group: &str, folder: &Path) -> Result<(), Error> {
        let Self { new, kept, pending } = self;
        config.print(
            &format!("{group}\t{new}\t{kept}\t{pending}\t{}", folder.display()),
            &json!({
                "grupo": group, "baixadas": new, "ja-havia": kept, "pendentes": pending,
                "pasta": folder,
            }),
        )
    }
}

/// The file a download is written to until it is complete.
#[must_use = "the path is only useful to write the download to"]
pub(crate) fn part_path(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".part");
    name.into()
}

/// What came of asking for one image.
#[derive(Debug)]
pub(crate) enum Fetched {
    /// The image is in the folder.
    Image(Inspected),
    /// The account could not get the image, for this reason.
    Pending(String),
}

/// Finishes one image, given what FamilySearch answered when asked for it:
/// transfers it from the redirect address, when there is one, checks it and
/// puts it in `file`, which only exists once the image is whole. A refusal
/// of this one image, or something that is not an image, leaves it pending.
pub(crate) fn keep(
    transport: &Transport,
    asked: Result<Option<String>, Error>,
    file: &Path,
) -> Result<Fetched, Error> {
    let part = part_path(file);
    let transferred = asked.and_then(|address| match address {
        Some(address) => transport.transfer(&address, &part),
        None => Ok(()),
    });
    let kept = match transferred.and_then(|()| inspect(&part)) {
        Ok(Some(image)) => {
            fs::rename(&part, file)
                .with_context(|| format!("não consegui gravar {}", file.display()))?;
            return Ok(Fetched::Image(image));
        }
        Ok(None) => Ok(Fetched::Pending(
            "o que veio não é uma imagem inteira".to_owned(),
        )),
        Err(error @ Error::Restricted { .. }) => Ok(Fetched::Pending(error.to_string())),
        Err(error) => Err(error),
    };
    // What a download that did not complete left behind, if anything.
    let _ = fs::remove_file(&part);
    kept
}

/// The images still to ask for, shared by the workers of a download.
#[derive(Debug)]
struct Queue {
    /// The position, among the wanted images, of the next one to take.
    next: usize,
    /// Set once a request fails: no worker asks for anything after that.
    stopped: bool,
}

/// Downloads the full images of a film into the folder, several transfers at
/// a time, recording each one in the manifest as it completes.
fn download(
    config: &Config,
    film: &Film,
    range: &RangeArgs,
    folder: &Path,
    workers: usize,
    retry_pending: bool,
) -> Result<(), Error> {
    let transport = Transport::new(config);
    let mut manifest = Manifest::open(folder)?;
    let group = film.to_string();
    let numbers = numbers(range, total(&transport, &mut manifest, film)?)?;
    let mut tally = Tally::default();
    let mut wanted = Vec::new();
    for number in numbers {
        match settled(folder, manifest.image(&group, number), retry_pending) {
            Some(settled) => tally.settle(settled),
            None => wanted.push(number),
        }
    }
    let queue = Mutex::new(Queue {
        next: 0,
        stopped: false,
    });
    let stop = || queue.lock().expect("download queue lock poisoned").stopped = true;
    let failure = thread::scope(|scope| {
        let (results, inbox) = mpsc::sync_channel(workers);
        for _ in 0..workers.min(wanted.len()) {
            let (results, transport, wanted, queue, stop) =
                (results.clone(), &transport, &wanted, &queue, &stop);
            scope.spawn(move || {
                loop {
                    // The lock is the line for the account's turn: it is
                    // held while the image is asked of FamilySearch, so the
                    // requests go one at a time and none follows a failure.
                    // The transfer comes after it, beside the others.
                    let (number, file, asked) = {
                        let mut queue = queue.lock().expect("download queue lock poisoned");
                        let Some(&number) = wanted.get(queue.next).filter(|_| !queue.stopped)
                        else {
                            break;
                        };
                        queue.next += 1;
                        let file = folder.join(file_name(film, number));
                        let route = format!("/{}/dist.jpg", node(film, number));
                        let asked = transport.ask_image(&route, &part_path(&file));
                        // Only the refusal of this one image lets the rest go on.
                        queue.stopped =
                            matches!(&asked, Err(error) if !matches!(error, Error::Restricted { .. }));
                        (number, file, asked)
                    };
                    let fetched = keep(transport, asked, &file);
                    if fetched.is_err() {
                        stop();
                    }
                    // The receiver only goes away once every worker is done.
                    if results.send((number, fetched)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(results);
        let mut failure = None;
        // This thread alone writes the manifest.
        for (number, fetched) in inbox {
            let recorded = fetched.and_then(|fetched| {
                let id = node(film, number);
                manifest.record(match fetched {
                    Fetched::Image(image) => {
                        tally.new += 1;
                        Line::Downloaded {
                            group: group.clone(),
                            number,
                            id,
                            file: file_name(film, number),
                            sha256: image.sha256,
                            bytes: image.bytes,
                            width: image.width,
                            height: image.height,
                        }
                    }
                    Fetched::Pending(reason) => {
                        tally.pending += 1;
                        eprintln!("garimpo: imagem {number} pendente de acesso: {reason}");
                        Line::Pending {
                            group: group.clone(),
                            number,
                            id,
                            reason,
                        }
                    }
                })
            });
            match recorded {
                Ok(()) if tally.new > 0 && tally.new % PROGRESS_EVERY == 0 => {
                    eprintln!(
                        "garimpo: {} de {} imagens baixadas",
                        tally.new,
                        wanted.len()
                    );
                }
                Ok(()) => {}
                Err(error) => {
                    stop();
                    failure.get_or_insert(error);
                }
            }
        }
        failure
    });
    manifest.compact()?;
    if let Some(failure) = failure {
        eprintln!(
            "garimpo: parou com {} imagens baixadas nesta rodada; repita o mesmo comando para \
             retomar",
            tally.new
        );
        return Err(failure);
    }
    tally.print(config, &group, folder)
}

/// Downloads the thumbnails of a film into the folder, for triage. A
/// thumbnail that is already there is not asked for again.
fn thumbnails(config: &Config, film: &Film, range: &RangeArgs, folder: &Path) -> Result<(), Error> {
    let transport = Transport::new(config);
    let mut manifest = Manifest::open(folder)?;
    let numbers = numbers(range, total(&transport, &mut manifest, film)?)?;
    let folder = folder.join("miniaturas");
    fs::create_dir_all(&folder)
        .with_context(|| format!("não consegui criar a pasta {}", folder.display()))?;
    let mut tally = Tally::default();
    for number in numbers {
        let file = folder.join(file_name(film, number));
        if file.exists() {
            tally.kept += 1;
            continue;
        }
        let route = format!("/{}/thumb_p200.jpg", node(film, number));
        let asked = transport.ask_image(&route, &part_path(&file));
        match keep(&transport, asked, &file)? {
            Fetched::Image(_) => tally.new += 1,
            Fetched::Pending(reason) => {
                tally.pending += 1;
                eprintln!("garimpo: miniatura {number} pendente de acesso: {reason}");
            }
        }
    }
    tally.print(config, &film.to_string(), &folder)
}
