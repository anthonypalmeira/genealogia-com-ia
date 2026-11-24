//! Text comparison that ignores accents, case and line breaks: the same name
//! shows up in several spellings, and a transcription breaks lines anywhere.

/// How many characters of context a passage keeps on each side of the term.
const CONTEXT: usize = 80;

/// The letter without its accent, for the accents of Portuguese and Spanish.
#[must_use = "the conversion has no effect of its own"]
pub(crate) fn unaccent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        'ñ' => 'n',
        'Ñ' => 'N',
        other => other,
    }
}

/// The text with single spaces, in lower case and without accents, one
/// character for each character of the spaced text.
fn fold(text: &str) -> impl Iterator<Item = char> {
    spaced(text).map(|c| unaccent(c.to_lowercase().next().unwrap_or(c)))
}

/// The text with every run of white space turned into one space.
fn spaced(text: &str) -> impl Iterator<Item = char> {
    text.split_whitespace()
        .enumerate()
        .flat_map(|(n, word)| (n > 0).then_some(' ').into_iter().chain(word.chars()))
}

/// Whether `text` holds `term`, ignoring accents, case and line breaks.
#[must_use = "the comparison has no effect of its own"]
pub(crate) fn holds(text: &str, term: &str) -> bool {
    Searchable::new(text).find(term).is_some()
}

/// A text prepared to be searched many times.
#[derive(Debug)]
pub(crate) struct Searchable {
    spaced: Vec<char>,
    folded: Vec<char>,
}

impl Searchable {
    #[must_use = "preparing a text has no effect of its own"]
    pub(crate) fn new(text: &str) -> Self {
        Self {
            spaced: spaced(text).collect(),
            folded: fold(text).collect(),
        }
    }

    /// Where `term` first occurs, as a range of characters.
    fn find(&self, term: &str) -> Option<(usize, usize)> {
        let term: Vec<char> = fold(term).collect();
        if term.is_empty() {
            return None;
        }
        let start = self
            .folded
            .windows(term.len())
            .position(|window| window == term)?;
        Some((start, start + term.len()))
    }

    /// The passage around the first occurrence of `term`, if it occurs.
    #[must_use = "the passage is only useful when shown"]
    pub(crate) fn passage(&self, term: &str) -> Option<String> {
        let (start, end) = self.find(term)?;
        Some(self.cut(
            start.saturating_sub(CONTEXT),
            (end + CONTEXT).min(self.spaced.len()),
        ))
    }

    /// The opening of the text, as long as a passage.
    #[must_use = "the opening is only useful when shown"]
    pub(crate) fn opening(&self) -> String {
        self.cut(0, (2 * CONTEXT).min(self.spaced.len()))
    }

    /// A stretch of the text, with `…` on each side that was cut.
    fn cut(&self, start: usize, end: usize) -> String {
        let mut passage = String::new();
        if start > 0 {
            passage.push('…');
        }
        passage.extend(&self.spaced[start..end]);
        if end < self.spaced.len() {
            passage.push('…');
        }
        passage
    }
}
