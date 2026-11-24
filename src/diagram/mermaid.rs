//! The Mermaid back end: a flowchart that reads upwards, from the youngest
//! person to the oldest ancestor, with the marks of the checks drawn in.

use serde_json::Value;

use super::{Grade, Role, check::Checked};

/// The class of each role, with its colors. Colors go in the drawing itself,
/// so any Mermaid viewer shows them.
const ROLES: [(Role, &str, &str); 3] = [
    (
        Role::Direct,
        "direto",
        "fill:#dbeafe,stroke:#1d4ed8,color:#111827",
    ),
    (
        Role::Collateral,
        "colateral",
        "fill:#f3f4f6,stroke:#6b7280,color:#111827",
    ),
    (
        Role::Context,
        "contexto",
        "fill:#ffffff,stroke:#9ca3af,color:#4b5563,stroke-dasharray:4 3",
    ),
];

/// The class of a person that breaks a rule.
const BROKEN: &str = "fill:#fee2e2,stroke:#dc2626,color:#991b1b,stroke-width:3px";

/// The color of a link, by what it is worth.
fn color(worth: Option<Grade>) -> &'static str {
    match worth {
        Some(Grade::A) => "#15803d",
        Some(Grade::B) => "#1d4ed8",
        Some(Grade::C) => "#b45309",
        Some(Grade::D) => "#c2410c",
        Some(Grade::F) => "#b91c1c",
        None => "#6b7280",
    }
}

/// A text as it goes inside a quoted Mermaid label.
fn label(text: &str) -> String {
    text.replace('"', "#quot;")
}

/// Draws the checked lineage as Mermaid.
#[must_use = "the drawing is only useful when written out"]
pub(super) fn draw(checked: &Checked) -> String {
    let lineage = &checked.lineage;
    let mut lines = Vec::new();
    let mut line = |text: String| lines.push(text);
    if let Some(title) = &lineage.title {
        // A JSON string is a YAML string too, whatever the title holds.
        line(format!("---\ntitle: {}\n---", Value::from(title.as_str())));
    }
    line("flowchart BT".to_owned());
    for (position, (person, broken)) in lineage
        .persons
        .iter()
        .zip(&checked.person_breaks)
        .enumerate()
    {
        let mut parts = vec![person.name.clone()];
        parts.extend(person.years());
        if !broken.is_empty() {
            parts.push(format!("⚠ {}", broken.join(", ")));
        }
        line(format!(
            "    p{position}[\"{}\"]",
            label(&parts.join("<br/>"))
        ));
    }
    for (position, hole) in checked.breaks.iter().enumerate() {
        line(format!("    furo{position}[\"⚠ {}\"]", label(hole)));
    }
    for (position, (link, broken)) in lineage.links.iter().zip(&checked.link_breaks).enumerate() {
        let arrow = match link.role {
            Role::Direct => "==>",
            Role::Collateral => "-->",
            Role::Context => "-.->",
        };
        let weakest = checked
            .weakest
            .contains(&position)
            .then(|| "▼ elo mais fraco".to_owned());
        let claim = if broken.is_empty() {
            vec![
                link.relation.clone(),
                link.grade.map(|grade| grade.to_string()),
                link.identity.map(|grade| format!("identidade {grade}")),
                link.document.clone(),
            ]
        } else {
            // A broken link shows what it breaks instead of what it claims.
            vec![Some(format!("⚠ {}", broken.join(", ")))]
        };
        let parts: Vec<String> = weakest
            .into_iter()
            .chain(claim.into_iter().flatten())
            .collect();
        line(format!(
            "    p{} {arrow}|\"{}\"| p{}",
            link.from,
            label(&parts.join(" · ")),
            link.to
        ));
    }
    for (_, class, style) in ROLES {
        line(format!("    classDef {class} {style}"));
    }
    line(format!("    classDef quebra {BROKEN}"));
    // Each person takes the role of the link that matters most.
    let role = |person: usize| {
        lineage
            .links
            .iter()
            .filter(|link| link.from == person || link.to == person)
            .map(|link| link.role)
            .min()
            .unwrap_or(Role::Context)
    };
    let nodes = |members: Vec<String>, class: &str| {
        (!members.is_empty()).then(|| format!("    class {} {class}", members.join(",")))
    };
    let sound = |person: &usize| checked.person_breaks[*person].is_empty();
    let persons = 0..lineage.persons.len();
    for (of, class, _) in ROLES {
        let members = persons
            .clone()
            .filter(|person| sound(person) && role(*person) == of)
            .map(|person| format!("p{person}"))
            .collect();
        if let Some(members) = nodes(members, class) {
            line(members);
        }
    }
    let broken = persons
        .filter(|person| !sound(person))
        .map(|person| format!("p{person}"))
        .chain((0..checked.breaks.len()).map(|hole| format!("furo{hole}")))
        .collect();
    if let Some(members) = nodes(broken, "quebra") {
        line(members);
    }
    for (position, (link, broken)) in lineage.links.iter().zip(&checked.link_breaks).enumerate() {
        let width = if checked.weakest.contains(&position) {
            5
        } else if broken.is_empty() {
            2
        } else {
            4
        };
        let style = if broken.is_empty() {
            format!("stroke:{},stroke-width:{width}px", color(link.worth()))
        } else {
            format!("stroke:#dc2626,stroke-width:{width}px,stroke-dasharray:6 4")
        };
        line(format!("    linkStyle {position} {style}"));
    }
    lines.join("\n") + "\n"
}
