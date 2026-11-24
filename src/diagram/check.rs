//! The rules a lineage has to keep, and the marks of what breaks them: every
//! link with a grade and a document, dates that are possible, one connected
//! direct line, and the weakest link of that line pointed out.

use super::{Lineage, Link, Role};

/// The youngest and the oldest a parent can plausibly be at a child's birth.
const PARENT_AGES: std::ops::RangeInclusive<i32> = 12..=80;

/// The longest life taken as possible, in years.
const LONGEST_LIFE: i32 = 120;

/// A lineage with the marks of its checks.
#[derive(Debug)]
pub(super) struct Checked {
    pub(super) lineage: Lineage,
    /// What each link breaks, in the order of `Lineage::links`.
    pub(super) link_breaks: Vec<Vec<String>>,
    /// What each person breaks, in the order of `Lineage::persons`.
    pub(super) person_breaks: Vec<Vec<String>>,
    /// What the lineage as a whole breaks.
    pub(super) breaks: Vec<String>,
    /// The direct line, from the youngest person up, as positions of links.
    pub(super) line: Vec<usize>,
    /// The links of the direct line that are worth the least.
    pub(super) weakest: Vec<usize>,
}

impl Checked {
    /// Runs every rule over the lineage.
    #[must_use = "the marks are only useful when drawn"]
    pub(super) fn new(lineage: Lineage) -> Self {
        let mut link_breaks = vec![Vec::new(); lineage.links.len()];
        let mut person_breaks = vec![Vec::new(); lineage.persons.len()];
        let mut breaks = Vec::new();
        for (link, broken) in lineage.links.iter().zip(&mut link_breaks) {
            if link.grade.is_none() {
                broken.push("sem nota".to_owned());
            }
            if link.document.is_none() {
                broken.push("sem documento".to_owned());
            }
            broken.extend(impossible_dates(&lineage, link));
        }
        for (person, broken) in lineage.persons.iter().zip(&mut person_breaks) {
            if let (Some(born), Some(died)) = (person.born, person.died) {
                if died < born {
                    broken.push("morreu antes de nascer".to_owned());
                } else if died - born > LONGEST_LIFE {
                    broken.push(format!("viveu mais de {LONGEST_LIFE} anos"));
                }
            }
        }
        let line = direct_line(&lineage);
        if line.is_empty() {
            breaks.push("nenhum elo direto: não há linha a desenhar".to_owned());
        }
        for (position, link) in lineage.links.iter().enumerate() {
            if link.role == Role::Direct && !line.contains(&position) {
                link_breaks[position]
                    .push("fora da linha direta, que segue por outro elo".to_owned());
            }
        }
        let tied = tied_to_line(&lineage, &line);
        for (broken, tied) in person_breaks.iter_mut().zip(tied) {
            // With no line at all, the lineage itself carries the mark.
            if !tied && !line.is_empty() {
                broken.push("sem ligação com a linha direta".to_owned());
            }
        }
        let worth = |position: &usize| lineage.links[*position].worth();
        let least = line.iter().filter_map(worth).max();
        let weakest = line
            .iter()
            .copied()
            .filter(|position| least.is_some() && worth(position) == least)
            .collect();
        Self {
            lineage,
            link_breaks,
            person_breaks,
            breaks,
            line,
            weakest,
        }
    }

    /// Every hole of the lineage, one per line, with whom it is about.
    #[must_use = "the holes are only useful when shown"]
    pub(super) fn holes(&self) -> Vec<String> {
        let Lineage { persons, links, .. } = &self.lineage;
        let of_links = links
            .iter()
            .zip(&self.link_breaks)
            .flat_map(|(link, broken)| {
                let (from, to) = (&persons[link.from].name, &persons[link.to].name);
                broken
                    .iter()
                    .map(move |hole| format!("{from} → {to}: {hole}"))
            });
        let of_persons = persons
            .iter()
            .zip(&self.person_breaks)
            .flat_map(|(person, broken)| {
                broken
                    .iter()
                    .map(move |hole| format!("{}: {hole}", person.name))
            });
        self.breaks
            .iter()
            .cloned()
            .chain(of_links)
            .chain(of_persons)
            .collect()
    }
}

/// What makes the dates of a child and a parent impossible together.
fn impossible_dates(lineage: &Lineage, link: &Link) -> Vec<String> {
    let (child, parent) = (&lineage.persons[link.from], &lineage.persons[link.to]);
    let mut broken = Vec::new();
    if link.relation.is_some() {
        return broken;
    }
    if let (Some(child_born), Some(parent_born)) = (child.born, parent.born) {
        let age = child_born - parent_born;
        if !PARENT_AGES.contains(&age) {
            broken.push(format!(
                "{} teria {age} anos quando {} nasceu",
                parent.name, child.name
            ));
        }
    }
    // A father can die up to a year before the birth.
    if let (Some(child_born), Some(parent_died)) = (child.born, parent.died)
        && child_born > parent_died + 1
    {
        broken.push(format!(
            "{} nasceu depois da morte de {}",
            child.name, parent.name
        ));
    }
    broken
}

/// The direct line: the direct links that follow one another from the
/// youngest person up, as positions in `Lineage::links`. A direct link that is
/// not on it belongs to a line that splits or that does not connect.
fn direct_line(lineage: &Lineage) -> Vec<usize> {
    let direct = || {
        lineage
            .links
            .iter()
            .enumerate()
            .filter(|(_, link)| link.role == Role::Direct)
    };
    // The line starts at somebody who is a child and nobody's parent.
    let start = direct().find(|(_, link)| direct().all(|(_, other)| other.to != link.from));
    let mut line = Vec::new();
    let mut on_line = Vec::new();
    let mut person = start.map(|(_, link)| link.from);
    while let Some(child) = person {
        on_line.push(child);
        // A link back to somebody already on the line would close a circle:
        // it stays off the line, and so it is marked.
        let next = direct().find(|(_, link)| link.from == child && !on_line.contains(&link.to));
        person = next.map(|(position, link)| {
            line.push(position);
            link.to
        });
    }
    line
}

/// Whether each person is tied, through links of any role, to the direct line.
fn tied_to_line(lineage: &Lineage, line: &[usize]) -> Vec<bool> {
    let mut tied = vec![false; lineage.persons.len()];
    for &position in line {
        let link = &lineage.links[position];
        tied[link.from] = true;
        tied[link.to] = true;
    }
    // Each pass ties whoever shares a link with somebody already tied.
    loop {
        let before = tied.iter().filter(|tied| **tied).count();
        for link in &lineage.links {
            if tied[link.from] || tied[link.to] {
                tied[link.from] = true;
                tied[link.to] = true;
            }
        }
        if tied.iter().filter(|tied| **tied).count() == before {
            return tied;
        }
    }
}
