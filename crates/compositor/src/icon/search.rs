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
//! Case-insensitive `contains`, over the name, then every tag and category, then
//! the names upstream retired. Not fuzzy, not scored. What is being optimised is
//! *the caller finds the icon*, and a scoring function nobody can predict is
//! what makes a miss inexplicable: a query that returns nothing should send
//! somebody to a different word, not to reading this file to work out why.
//!
//! The one ordering that is not alphabetical earns itself: an **exact name**
//! first, then names containing the word, then everything matched by a tag, then
//! everything matched only by a former name. A caller who already knows the name
//! is confirming it, and should not read past nine others to do so — least of
//! all past a name upstream has retired.
//!
//! ## Why a former name is searched, and why it is marked
//!
//! An alias is *literally the name the icon used to have*, which is the most
//! likely wrong guess there is: `unlock` is what the rest of the world calls the
//! icon Lucide now files as `lock-open`, and before this it found nothing at all.
//!
//! It comes back **marked** ([`Hit::formerly`]) because an unexplained alias hit
//! reads exactly like the fuzziness this search deliberately refuses — somebody
//! who typed `unlock` and got `lock-open` back with no reason beside it has no
//! way to tell a retired name from a scoring function. And it comes back
//! **last**, because a retired name is the weakest evidence in the record and a
//! caller who typed a current name must never be pushed down the list by one.
//!
//! What it never does is make the alias writable. [`super::find`] knows one name
//! per icon, validation refuses the rest, and the name in a [`Hit`] is always
//! the catalogue's own — the former name sits beside it as an explanation.

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
    /// The icons to show, best first — at most forty of them.
    pub hits: Vec<Hit>,
}

/// One icon a search found, and what in it matched.
///
/// The two fields are not two names: [`Hit::name`] is the catalogue's, and the
/// only string an `icon` asset's `name` will accept. [`Hit::formerly`] is an
/// explanation for why an icon called something else came back at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// The icon's own name — a string an `icon` asset's `name` will accept.
    pub name: &'static str,
    /// The retired name that matched, when nothing current in the record did.
    /// `None` for a hit on the name, a tag or a category, which is most of them.
    pub formerly: Option<&'static str>,
}

impl Search {
    /// Whether more matched than are shown here.
    pub fn is_capped(&self) -> bool {
        self.total > self.hits.len()
    }

    /// Just the names, best first — every one writable as an `icon` asset's
    /// `name`, the former names left behind.
    pub fn names(&self) -> Vec<&'static str> {
        self.hits.iter().map(|hit| hit.name).collect()
    }
}

impl fmt::Display for Hit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        match self.formerly {
            // The word is doing the explaining: `lock-open (formerly unlock)`
            // says which of the two to write without a legend anywhere.
            Some(formerly) => write!(f, " (formerly {formerly})"),
            None => Ok(()),
        }
    }
}

impl fmt::Display for Search {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.hits.is_empty() {
            // Zero is an answer, and it says what was searched so the reader
            // knows to try another word rather than that something broke.
            return write!(
                f,
                "No icon matches `{}`. All {} icon names, tags, categories and \
                 former names were searched — that is the answer and not a \
                 failure, so the thing to change is the word.",
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
            write!(f, ", of which the first {}", self.hits.len())?;
        }
        let listed: Vec<String> = self.hits.iter().map(Hit::to_string).collect();
        write!(f, ": {}.", listed.join(", "))?;
        if self.hits.iter().any(|hit| hit.formerly.is_some()) {
            // Said once, at the end, rather than beside each one: the reader has
            // to be told which of the two words to write, and told it exactly as
            // often as an alias actually turned up.
            write!(
                f,
                " A name in brackets is one upstream retired — write the name \
                 before it."
            )?;
        }
        if self.is_capped() {
            write!(f, " Narrow the word to see the rest.")?;
        }
        Ok(())
    }
}

/// Every icon whose name, tags, categories or former names contain this word.
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
            hits: Vec::new(),
        };
    }

    let mut ranked: Vec<(u8, Hit)> = catalogue::all()
        .iter()
        .filter_map(|icon| rank(&query, icon))
        .collect();
    // Stable, so the catalogue's own name order survives inside each rank —
    // the same query always answers with the same list in the same order.
    ranked.sort_by_key(|(rank, _)| *rank);

    let total = ranked.len();
    Search {
        query,
        total,
        hits: ranked.into_iter().take(MOST).map(|(_, hit)| hit).collect(),
    }
}

/// How good a match this icon is, and what matched — or `None` for one that
/// does not match at all.
///
/// Lower is better, and the four answers are the four kinds of evidence in the
/// record, strongest first. A former name is last of them however exactly it
/// matched: an exact hit on a name upstream has retired is still weaker
/// evidence than a tag on the name it actually has, and a rank that made an
/// exception for it would push a current name down the list.
fn rank(query: &str, icon: &Icon) -> Option<(u8, Hit)> {
    let itself = |rank| {
        (
            rank,
            Hit {
                name: icon.name(),
                formerly: None,
            },
        )
    };
    if icon.name() == query {
        return Some(itself(0));
    }
    if contains(icon.name(), query) {
        return Some(itself(1));
    }
    if icon
        .tags()
        .iter()
        .chain(icon.categories())
        .any(|word| contains(word, query))
    {
        return Some(itself(2));
    }
    // The first alias that matched, in upstream's order: it is an explanation
    // and one is enough of one.
    let formerly = icon
        .aliases()
        .iter()
        .find(|former| contains(former, query))?;
    Some((
        3,
        Hit {
            name: icon.name(),
            formerly: Some(formerly),
        },
    ))
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
