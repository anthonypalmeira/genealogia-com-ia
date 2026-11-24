//! Waypoints: the tree that browses a collection with no index, from the
//! collection down to localities, volumes and the page images of each volume.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Found, Hit, Query, RouteId, Title, ark, count, read, report};
use crate::{
    cli::Config,
    coverage::{Kind, Search},
    error::Error,
    familysearch::field,
    transport::Transport,
};

/// One level of the tree. Everything comes as source descriptions: the
/// document names its node, and each child points at it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Level {
    /// The reference to the node of this level, such as `#src_1`.
    description: String,
    source_descriptions: Vec<Description>,
    /// Under `self`, `results` counts the children of the node.
    #[serde(default)]
    links: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Description {
    /// The address of what is described: a waypoint or the ARK of an image.
    about: String,
    component_of: Option<Reference>,
    resource_type: Option<String>,
    #[serde(default)]
    titles: Vec<Title>,
}

#[derive(Debug, Deserialize)]
struct Reference {
    description: String,
}

/// Lists one level of a collection's waypoints: the first one, or the one
/// under `waypoint`.
pub(super) fn list_level(
    config: &Config,
    transport: &Transport,
    collection: &RouteId,
    waypoint: Option<&RouteId>,
    count_asked: u32,
    offset: u32,
    log: &Path,
) -> Result<(), Error> {
    let page = Query::new()
        .with("count", Some(count_asked))
        .with("start", Some(offset));
    // A waypoint is only found under the collection it was listed in.
    let path = match waypoint {
        Some(waypoint) => format!(
            "/platform/records/waypoints/{waypoint}?{}",
            page.with("cc", Some(collection)).finish()
        ),
        None => format!(
            "/platform/records/collections/{collection}/waypoints?{}",
            page.finish()
        ),
    };
    let query = match waypoint {
        Some(waypoint) => format!("colecao:{collection} waypoint:{waypoint}"),
        None => format!("colecao:{collection}"),
    };
    let search = Search {
        offset: offset.into(),
        ..Search::new(Kind::Waypoints, query)
    };
    report(config, log, search, || {
        // The official host answers these routes, with its own media type.
        let level: Level = read(&transport.get(&path)?, "/platform/records/waypoints")?;
        let hits: Vec<Hit> = level
            .source_descriptions
            .iter()
            .filter(|child| {
                child
                    .component_of
                    .as_ref()
                    .is_some_and(|parent| parent.description == level.description)
            })
            .zip(u64::from(offset) + 1..)
            .map(|(child, position)| child_hit(child, position))
            .collect();
        Ok(Found {
            total: level
                .links
                .pointer("/self/results")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| count(&hits)),
            heading: None,
            hits,
        })
    })
}

/// One child of a waypoint: a page image, or a level to descend into.
fn child_hit(child: &Description, position: u64) -> Hit {
    // What comes after `marker` in the child's address, up to its query.
    let after = |marker: &str| {
        let (_, rest) = child.about.split_once(marker)?;
        rest.split('?').next()
    };
    let is_image = child
        .resource_type
        .as_ref()
        .is_some_and(|kind| kind.ends_with("DigitalArtifact"));
    if is_image {
        let id = ark(after("ark:/61903/").unwrap_or_default());
        Hit {
            line: ["imagem".to_owned(), position.to_string(), field(Some(&id))].join("\t"),
            json: json!({"tipo": "imagem", "posicao": position, "id": id}),
            id,
        }
    } else {
        let id = after("/waypoints/").unwrap_or_default().to_owned();
        let title = child.titles.first().map(|title| &title.value);
        Hit {
            line: ["waypoint".to_owned(), field(Some(&id)), field(title)].join("\t"),
            json: json!({"tipo": "waypoint", "id": id, "titulo": title}),
            id,
        }
    }
}
