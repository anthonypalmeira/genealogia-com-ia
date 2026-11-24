//! The plain-text back end: the same marked lineage, to paste where a drawing
//! does not go, such as a chat message.

use super::{Link, Person, check::Checked};

/// A person with the years, as `Nome (1820–1880)`.
fn named(person: &Person) -> String {
    match person.years() {
        Some(years) => format!("{} ({years})", person.name),
        None => person.name.clone(),
    }
}

/// What a sound link claims: the grades and the document.
fn claim(link: &Link) -> String {
    [
        link.grade.map(|grade| format!("nota {grade}")),
        link.identity.map(|grade| format!("identidade {grade}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ")
        + &link
            .document
            .as_ref()
            .map(|document| format!(" — {document}"))
            .unwrap_or_default()
}

/// Writes the checked lineage as plain text.
#[must_use = "the drawing is only useful when written out"]
pub(super) fn draw(checked: &Checked) -> String {
    let lineage = &checked.lineage;
    let mut lines = Vec::new();
    let mut line = |text: String| lines.push(text);
    // What a link shows: its breaks when it has any, its claim otherwise.
    let shown = |position: usize| {
        let broken = &checked.link_breaks[position];
        if broken.is_empty() {
            claim(&lineage.links[position])
        } else {
            format!("⚠ {}", broken.join(", "))
        }
    };
    if let Some(title) = &lineage.title {
        line(format!("{title}\n"));
    }
    line("Linha direta".to_owned());
    for (step, &position) in checked.line.iter().enumerate() {
        let link = &lineage.links[position];
        if step == 0 {
            line(format!("1. {}", named(&lineage.persons[link.from])));
        }
        let weakest = if checked.weakest.contains(&position) {
            " ◀ elo mais fraco"
        } else {
            ""
        };
        line(format!("   ↑ {}{weakest}", shown(position)));
        line(format!(
            "{}. {}",
            step + 2,
            named(&lineage.persons[link.to])
        ));
    }
    let others: Vec<usize> = (0..lineage.links.len())
        .filter(|position| !checked.line.contains(position))
        .collect();
    if !others.is_empty() {
        line("\nColaterais e contexto".to_owned());
    }
    for position in others {
        let link = &lineage.links[position];
        let relation = link.relation.as_deref().unwrap_or("filho(a)");
        line(format!(
            "- {}, {relation} de {}: {}",
            named(&lineage.persons[link.from]),
            lineage.persons[link.to].name,
            shown(position)
        ));
    }
    let holes = checked.holes();
    if !holes.is_empty() {
        line("\nFuros".to_owned());
    }
    for hole in holes {
        line(format!("- {hole}"));
    }
    lines.join("\n") + "\n"
}
