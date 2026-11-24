//! The dates of persons, as precise as they were given, and what makes the
//! dates of a child and of a parent impossible together: one rule for the
//! tree and for the drawing of a lineage.

use std::{fmt, ops::RangeInclusive};

use gedcomx::{FactType, Person, gedcomx_date::GedcomxDate};

/// The youngest and the oldest a parent can plausibly be at a child's birth.
const PARENT_AGES: RangeInclusive<i64> = 12..=80;

/// A date as precise as it was given: a year, a year and a month, or a day.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Date {
    year: i32,
    month: Option<u32>,
    /// Only with a month.
    day: Option<u32>,
}

impl Date {
    /// The date of the first fact of a kind that has one, as the tree gives
    /// it: the standard form or, without one, the year in what the
    /// contributor typed.
    #[must_use = "the date is only useful when read"]
    pub(crate) fn of(person: &Person, kind: &FactType) -> Option<Self> {
        let facts = person.facts.iter().filter(|fact| fact.fact_type == *kind);
        facts
            .filter_map(|fact| fact.date.as_ref())
            .find_map(|date| match &date.formal {
                Some(formal) => Self::standard(formal.0),
                None => Self::typed(date.original.as_deref()?),
            })
    }

    /// The date of a standard form that is one date; an approximate date
    /// counts as the date it is about. A range ("before 1800", "between 1799
    /// and 1801") is not one date: taking a bound of it for the date would
    /// stop a traversal or mark a link on a guess.
    fn standard(formal: GedcomxDate) -> Option<Self> {
        let date = match formal {
            GedcomxDate::Simple(simple) => simple.date,
            GedcomxDate::Range(_) | GedcomxDate::Recurring(_) => return None,
        };
        Some(Self {
            year: date.year,
            month: date.month,
            day: date.month.and(date.day),
        })
    }

    /// The year in a date as somebody typed it: its first run of four digits.
    fn typed(text: &str) -> Option<Self> {
        let year = text
            .split(|c: char| !c.is_ascii_digit())
            .find(|digits| digits.len() == 4)?;
        Some(Self {
            year: year.parse().ok()?,
            month: None,
            day: None,
        })
    }

    /// A date of which only the year is known.
    #[must_use = "the date is only useful when compared"]
    pub(crate) fn of_year(year: i32) -> Self {
        Self {
            year,
            month: None,
            day: None,
        }
    }

    #[must_use = "the year is only useful when compared"]
    pub(crate) fn year(self) -> i32 {
        self.year
    }

    /// The year, the month and the day, with no more precision than `other`
    /// has: 0 stands for what either date lacks, so two dates cut by each
    /// other compare part by part.
    fn at_precision_of(self, other: Self) -> [i64; 3] {
        let month = other.month.and(self.month);
        let day = month.and(other.day).and(self.day);
        [
            self.year.into(),
            month.unwrap_or(0).into(),
            day.unwrap_or(0).into(),
        ]
    }

    /// The whole years from this date to a later one, at the precision both
    /// have: between two bare years, their difference.
    fn years_until(self, later: Self) -> i64 {
        let [year, month, day] = self.at_precision_of(later);
        let [later_year, later_month, later_day] = later.at_precision_of(self);
        later_year - year - i64::from((later_month, later_day) < (month, day))
    }
}

/// Whom a date rule is about: a child or a parent of a link.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lifespan<'a> {
    pub(crate) name: &'a str,
    pub(crate) born: Option<Date>,
    pub(crate) died: Option<Date>,
}

/// What makes the dates of a child and of a parent impossible together, at
/// the precision both dates have.
#[must_use = "the breaks are only useful when shown"]
pub(crate) fn impossible_together(child: Lifespan<'_>, parent: Lifespan<'_>) -> Vec<String> {
    let mut broken = Vec::new();
    if let (Some(child_born), Some(parent_born)) = (child.born, parent.born) {
        let age = parent_born.years_until(child_born);
        if !PARENT_AGES.contains(&age) {
            broken.push(format!(
                "{} teria {age} anos quando {} nasceu",
                parent.name, child.name
            ));
        }
    }
    // A father can die up to a year before the birth.
    if let (Some(child_born), Some(parent_died)) = (child.born, parent.died) {
        let [year, month, day] = parent_died.at_precision_of(child_born);
        if child_born.at_precision_of(parent_died) > [year + 1, month, day] {
            broken.push(format!(
                "{} nasceu depois da morte de {}",
                child.name, parent.name
            ));
        }
    }
    broken
}

/// The date as `1850-03-10`, `1850-03` or `1850`.
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.year)?;
        if let Some(month) = self.month {
            write!(f, "-{month:02}")?;
        }
        if let Some(day) = self.day {
            write!(f, "-{day:02}")?;
        }
        Ok(())
    }
}
