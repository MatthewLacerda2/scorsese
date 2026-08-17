//! Finding an icon by what it is about, when nobody knows what it is called.
//!
//! **Seventeen hundred names cannot go in a context window**, so the way an
//! assistant reaches this catalogue is by asking for a word. It knows it wants
//! the film-camera one; it does not know whether upstream calls that `film`,
//! `video`, `camera` or `clapperboard`, and without a search the only way to
//! find out is to guess a name, have validation refuse it, and guess again.
//!
//! Sibling of [`super::nearest`] and deliberately not the same question. That
//! one answers *this name was nearly right* — a typo, a half-remembered
//! compound — and it matches names only. This one answers *what is this icon
//! about*, which is what the tags and categories are for: `clapperboard` is
//! filed under *movie, film, cinema, camera* and eight more, none of which is
//! its name, so a search over names alone would miss it for every word anyone
//! would actually type.
//!
//! ## A substring, and nothing cleverer
//!
//! Case-insensitive `contains`, over the name and then over every tag and
//! category. Not fuzzy, not scored. What is being optimised is *the caller
//! finds the icon*, and a scoring function nobody can predict is what makes a
//! miss inexplicable: a query that returns nothing should send somebody to a
//! different word, not to reading this file to work out why.
//!
//! The one ordering that is not alphabetical earns itself: an **exact name**
//! first, then names containing the word, then everything matched by a tag. A
//! caller who already knows the name is confirming it, and should not read past
//! nine others to do so.

use std::fmt;

use super::{Icon, catalogue};

/// How many names one answer carries.
///
/// Chosen against the set rather than picked round: a real word — *camera*,
/// *music*, *warning*, *video* — matches fewer than forty of these icons, so
/// the cap bites only on a query too vague to answer in full, where the honest
/// reply is the count and a suggestion to narrow it. Forty hyphenated names is
/// also a couple of lines rather than a page, which is what keeps this cheap
/// enough to call on a hunch.
const MOST: usize = 40;

/// What a search over the catalogue found.
///
/// The count and the names are separate on purpose. A truncated answer that
/// printed like a complete one would read as *that is all there is*, which for
/// this tool is a wrong answer rather than a short one.
#[derive(Debug, Clone)]
pub struct Search {
    /// The word that was matched on: the query, trimmed and lowercased.
    pub query: String,
    /// How many icons matched in all, before the cap.
    pub total: usize,
    /// The names to show, best first — at most forty of them, and every one a
    /// string an `icon` asset's `name` will accept.
    pub names: Vec<&'static str>,
}

impl Search {
    /// Whether more matched than are named here.
    pub fn is_capped(&self) -> bool {
        self.total > self.names.len()
    }
}

impl fmt::Display for Search {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.names.is_empty() {
            // Zero is an answer, and it says what was searched so the reader
            // knows to try another word rather than that something broke.
            return write!(
                f,
                "No icon matches `{}`. All {} icon names, tags and categories \
                 were searched — that is the answer and not a failure, so the \
                 thing to change is the word.",
                self.query,
                catalogue::all().len()
            );
        }
        if self.total == 1 {
            write!(f, "1 icon matches `{}`", self.query)?;
        } else {
            write!(f, "{} icons match `{}`", self.total, self.query)?;
        }
        if self.is_capped() {
            write!(f, ", of which the first {}", self.names.len())?;
        }
        write!(f, ": {}.", self.names.join(", "))?;
        if self.is_capped() {
            write!(f, " Narrow the word to see the rest.")?;
        }
        Ok(())
    }
}

/// Every icon whose name, tags or categories contain this word.
///
/// An empty query finds nothing rather than everything: *all of them* is not an
/// answer to *which icon is this*, and a caller with no word to search on has
/// asked nothing yet.
pub(super) fn search(query: &str) -> Search {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Search {
            query,
            total: 0,
            names: Vec::new(),
        };
    }

    let mut ranked: Vec<(u8, &'static str)> = catalogue::all()
        .iter()
        .filter_map(|icon| rank(&query, icon).map(|rank| (rank, icon.name())))
        .collect();
    // Stable, so the catalogue's own name order survives inside each rank —
    // the same query always answers with the same list in the same order.
    ranked.sort_by_key(|(rank, _)| *rank);

    let total = ranked.len();
    Search {
        query,
        total,
        names: ranked
            .into_iter()
            .take(MOST)
            .map(|(_, name)| name)
            .collect(),
    }
}

/// How good a match this icon is, or `None` for one that does not match at all.
/// Lower is better, and the three answers are the three kinds of evidence.
fn rank(query: &str, icon: &Icon) -> Option<u8> {
    if icon.name() == query {
        return Some(0);
    }
    if contains(icon.name(), query) {
        return Some(1);
    }
    icon.tags()
        .iter()
        .chain(icon.categories())
        .any(|word| contains(word, query))
        .then_some(2)
}

/// `contains`, ignoring case, without allocating a lowercase copy of every tag.
///
/// Seventeen hundred icons carry ten thousand tags between them, and this runs
/// over all of them per query. ASCII-insensitive is the whole of what is needed:
/// upstream's words are ASCII, and the query arrives lowercased already, so a
/// byte that is not ASCII compares exactly and is none the worse for it.
fn contains(haystack: &str, needle: &str) -> bool {
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    haystack.len() >= needle.len()
        && haystack
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
}
