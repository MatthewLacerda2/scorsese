//! What to say when a document names a symbol that is not here.
//!
//! **"There is no icon called `clapperbord`" is half an answer**, and the worse
//! half: seventeen hundred names cannot be listed the way the shipped faces can,
//! so a refusal that only refuses leaves an author with nowhere to go. The other
//! half is the near ones, which is what this produces.
//!
//! Two ways of being near, because two mistakes are being caught and they are
//! not the same one:
//!
//! - **A typo** — `clapperbord` for `clapperboard`, `arow` for `arrow`. That is
//!   an edit distance, and a small one. `clapboard` is deliberately *not* an
//!   example: it is three edits from the clapperboard and one from the
//!   clipboard, so the clipboard is the honest answer.
//! - **A half-remembered name** — `play` when the set has `play`, `circle-play`
//!   and `square-play`, or `clapper` when it has `clapperboard`. Upstream's
//!   names are compounds, so the word somebody remembers is usually *inside*
//!   the name or the start of one rather than close to the whole of it, and an
//!   edit distance never finds those: `circle-play` is seven edits from `play`.
//!
//! Deliberately not a search. Finding an icon by what it is *about* — a tag, a
//! category, "something filmic" — is a different question with a different
//! answer, and it is filed as its own work. This is the answer to a name that
//! was nearly right.

use super::catalogue;
use crate::distance;

/// How many names are worth putting in one line of a refusal.
const MOST: usize = 6;

/// The furthest a name can be and still read as the same word mistyped. Two is
/// a transposed pair of letters, which is the typo worth catching; past that
/// the suggestion is a guess, and a wrong suggestion is worse than none
/// because it is the one thing a reader acts on without checking.
const NEAREST: usize = 2;

/// The names closest to one the catalogue does not have, best first.
///
/// Empty when nothing is close, which is an honest answer and reads as one: a
/// name with no near neighbours is not a typo, it is a symbol this set does not
/// have.
pub(super) fn nearest(name: &str) -> Vec<&'static str> {
    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return Vec::new();
    }

    let mut ranked: Vec<(usize, &'static str)> = catalogue::all()
        .iter()
        .filter_map(|icon| rank(&name, icon.name()).map(|rank| (rank, icon.name())))
        .collect();
    // By closeness, then alphabetically, so the same wrong name always produces
    // the same line — a refusal that reordered itself between runs would be a
    // diff nobody wrote.
    ranked.sort_unstable_by_key(|(rank, candidate)| (*rank, *candidate));
    ranked.truncate(MOST);
    ranked.into_iter().map(|(_, candidate)| candidate).collect()
}

/// The shortest thing that may be treated as the start of a word. Below it a
/// prefix says almost nothing — two letters would match a hundred names.
const SHORTEST_PREFIX: usize = 4;

/// How near a candidate is, or `None` for one that is not near at all. Lower is
/// nearer, and the order of the four answers is the order of the evidence.
///
/// A whole word beats every edit distance that is not exact, because it is the
/// stronger evidence: `circle-play` containing `play` whole is somebody who
/// knows the word and not the compound, where three edits apart is a
/// coincidence about as often as it is a typo. A *start* of a word comes last,
/// because `clapper` for `clapperboard` is a real thing to write and a weaker
/// signal than either.
fn rank(name: &str, candidate: &'static str) -> Option<usize> {
    if candidate == name {
        return Some(0);
    }
    if words(candidate).any(|word| word == name) {
        return Some(1);
    }
    let apart = distance::between(name, candidate);
    // Short names are exempt from the second half: at four letters, two edits
    // is half the word, and everything would be a near match for everything.
    if apart <= NEAREST && apart * 2 < name.len() {
        return Some(apart + 1);
    }
    let begins =
        name.len() >= SHORTEST_PREFIX && words(candidate).any(|word| word.starts_with(name));
    begins.then_some(NEAREST + 2)
}

/// The words a compound name is made of.
///
/// On the hyphens rather than anywhere in the string, so `play` matches
/// `circle-play` and `arrow` does not match `narrow-something`: a run of
/// letters in the middle of a word is a coincidence, and a word of a compound
/// is a name somebody half-remembered.
fn words(candidate: &'static str) -> impl Iterator<Item = &'static str> {
    candidate.split('-')
}
