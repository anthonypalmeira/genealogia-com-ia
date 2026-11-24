//! The manifest of an image folder: one line of JSON per image asked for,
//! with its hash or with why access to it is pending, so a download resumes
//! without losing or repeating anything.

use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions, TryLockError},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// One line of the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "situacao")]
pub(crate) enum Line {
    /// How many images a group has.
    #[serde(rename = "listado")]
    Listed {
        #[serde(rename = "grupo")]
        group: String,
        total: u32,
    },
    /// An image that is in the folder.
    #[serde(rename = "baixada")]
    Downloaded {
        #[serde(rename = "grupo")]
        group: String,
        #[serde(rename = "numero")]
        number: u32,
        /// What the image is called where it came from.
        id: String,
        /// The name of the file, inside the folder.
        #[serde(rename = "arquivo")]
        file: String,
        sha256: String,
        bytes: u64,
        #[serde(rename = "largura")]
        width: u32,
        #[serde(rename = "altura")]
        height: u32,
    },
    /// An image the account could not get. Not a missing image.
    #[serde(rename = "pendente-de-acesso")]
    Pending {
        #[serde(rename = "grupo")]
        group: String,
        #[serde(rename = "numero")]
        number: u32,
        id: String,
        #[serde(rename = "motivo")]
        reason: String,
    },
}

impl Line {
    /// The group and the number of the image; the line of a group's total
    /// comes before its images, as number 0.
    fn key(&self) -> (String, u32) {
        match self {
            Self::Listed { group, .. } => (group.clone(), 0),
            Self::Downloaded { group, number, .. } | Self::Pending { group, number, .. } => {
                (group.clone(), *number)
            }
        }
    }
}

/// The manifest of a folder, open for images to be recorded.
#[derive(Debug)]
pub(crate) struct Manifest {
    /// Holds the folder until the command ends: the manifest is rewritten
    /// whole from what this process read, so two commands on one folder take
    /// turns.
    _lock: File,
    path: PathBuf,
    file: File,
    /// The latest line of each image.
    lines: BTreeMap<(String, u32), Line>,
}

impl Manifest {
    /// Opens the manifest of `folder`, creating both when they do not exist.
    pub(crate) fn open(folder: &Path) -> Result<Self, Error> {
        let path = folder.join("manifesto.jsonl");
        let context = || format!("não consegui usar o manifesto {}", path.display());
        fs::create_dir_all(folder).with_context(context)?;
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(folder.join("manifesto.lock"))
            .with_context(context)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                eprintln!(
                    "garimpo: aguardando outro comando garimpo que usa a pasta {}",
                    folder.display()
                );
                lock.lock().with_context(context)?;
            }
            Err(TryLockError::Error(error)) => {
                return Err(anyhow::Error::new(error).context(context()).into());
            }
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
            Err(error) => return Err(anyhow::Error::new(error).context(context()).into()),
        };
        let mut lines = BTreeMap::new();
        for line in content.lines() {
            let line: Line = serde_json::from_str(line).with_context(context)?;
            lines.insert(line.key(), line);
        }
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .with_context(context)?;
        Ok(Self {
            _lock: lock,
            path,
            file,
            lines,
        })
    }

    /// How many images the group has, if it was ever listed.
    #[must_use = "the total is only useful to tell the images there are"]
    pub(crate) fn total(&self, group: &str) -> Option<u32> {
        match self.lines.get(&(group.to_owned(), 0)) {
            Some(Line::Listed { total, .. }) => Some(*total),
            Some(Line::Downloaded { .. } | Line::Pending { .. }) | None => None,
        }
    }

    /// The line of one image, if it was ever asked for.
    #[must_use = "the line is only useful to decide about the image"]
    pub(crate) fn image(&self, group: &str, number: u32) -> Option<&Line> {
        self.lines.get(&(group.to_owned(), number))
    }

    /// The lines of a group's images, in order.
    #[must_use = "the lines are only useful when read"]
    pub(crate) fn images(&self, group: &str) -> impl Iterator<Item = &Line> {
        self.lines
            .range((group.to_owned(), 1)..=(group.to_owned(), u32::MAX))
            .map(|(_, line)| line)
    }

    /// Records a line at once, so an interrupted command keeps what it did.
    /// A later line of the same image replaces the earlier one.
    pub(crate) fn record(&mut self, line: Line) -> Result<(), Error> {
        let json =
            serde_json::to_string(&line).context("não consegui gerar a linha do manifesto")?;
        self.file
            .write_all(format!("{json}\n").as_bytes())
            .with_context(|| format!("não consegui gravar o manifesto {}", self.path.display()))?;
        self.lines.insert(line.key(), line);
        Ok(())
    }

    /// Rewrites the manifest with one line per image, in order, replacing the
    /// file in one step.
    pub(crate) fn compact(&self) -> Result<(), Error> {
        let mut content = String::new();
        for line in self.lines.values() {
            content.push_str(
                &serde_json::to_string(line).context("não consegui gerar a linha do manifesto")?,
            );
            content.push('\n');
        }
        let draft = self.path.with_extension("jsonl.new");
        fs::write(&draft, content)
            .and_then(|()| fs::rename(&draft, &self.path))
            .with_context(|| format!("não consegui gravar o manifesto {}", self.path.display()))?;
        Ok(())
    }
}
