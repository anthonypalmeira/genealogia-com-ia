//! The portraits of the persons of a traversal, kept in a folder: what only
//! the image shows, such as a seal somebody put on it. A portrait is chosen
//! by the users of the tree, so it is a hint and never a proof.

use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::Path,
};

use anyhow::{Context, anyhow};
use serde_json::json;

use super::{PersonId, person_id, traversal::Traversal};
use crate::{
    cli::Config,
    error::Error,
    images::{Fetched, keep},
    transport::Transport,
};

/// Downloads into `folder` the portrait of each person of the traversal in
/// `file`, one request to FamilySearch per person. A portrait that is already
/// there is not asked for again, and neither is a person FamilySearch already
/// said to have none, which the folder keeps a list of.
pub(super) fn download(
    config: &Config,
    transport: &Transport,
    file: &Path,
    folder: &Path,
) -> Result<(), Error> {
    let traversal = Traversal::read(file)?;
    let list = folder.join("sem-retrato.txt");
    let context = || format!("não consegui usar a lista {}", list.display());
    fs::create_dir_all(folder).with_context(context)?;
    let listed = match fs::read_to_string(&list) {
        Ok(listed) => listed,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => return Err(anyhow::Error::new(error).context(context()).into()),
    };
    // A line that is not an id names nobody: whoever it was is asked again.
    let without: HashSet<PersonId> = listed.lines().filter_map(|id| id.parse().ok()).collect();
    let mut list = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&list)
        .with_context(context)?;
    let (mut new, mut kept, mut none, mut pending) = (0_u32, 0_u32, 0_u32, 0_u32);
    let persons = traversal
        .lines
        .iter()
        .filter_map(|line| person_id(&line.person));
    for person in persons {
        let portrait = folder.join(format!("{person}.jpg"));
        if portrait.exists() {
            kept += 1;
            continue;
        }
        if without.contains(&person) {
            none += 1;
            continue;
        }
        let route = format!("/platform/tree/persons/{person}/portrait");
        let reason = match transport.ask_portrait(&route) {
            Ok(None) => {
                // One write per person, so an interrupted command keeps what
                // it already asked.
                list.write_all(format!("{person}\n").as_bytes())
                    .with_context(context)?;
                none += 1;
                continue;
            }
            Ok(address) => match keep(transport, Ok(address), &portrait) {
                Ok(Fetched::Image(_)) => {
                    new += 1;
                    continue;
                }
                Ok(Fetched::Pending(reason)) => reason,
                // A refusal of this one address leaves the person pending. A 429
                // asks for a pause and a 5xx is the storage failing: both stop
                // the batch, so no other portrait is asked for meanwhile.
                Err(error @ Error::ImageStorage { status })
                    if (400..500).contains(&status) && status != 429 =>
                {
                    format!("{error} ao endereço fixo do retrato")
                }
                // The address is fixed: asking again gives the same one, so the
                // only remedy is to wait.
                Err(Error::ImageStorage { status }) => {
                    return Err(anyhow!(
                        "o armazenamento de retratos respondeu {status} ao endereço fixo do \
                         retrato de {person}; espere alguns minutos antes de rodar de novo"
                    )
                    .into());
                }
                Err(error) => return Err(error),
            },
            // An answer about this one person: merged, deleted or not found.
            Err(error @ Error::Input(_)) => error.to_string(),
            Err(error) => return Err(error),
        };
        // Not a missing portrait: the person is asked about again next time.
        eprintln!("garimpo: retrato de {person} pendente de acesso: {reason}");
        pending += 1;
    }
    config.print(
        &format!("{new}\t{kept}\t{none}\t{pending}\t{}", folder.display()),
        &json!({
            "baixados": new, "ja-havia": kept, "sem-retrato": none, "pendentes": pending,
            "pasta": folder,
        }),
    )
}
