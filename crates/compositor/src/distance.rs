//! How far apart two names are, for the two places that have to guess at one.
//!
//! A property path nobody animates and a symbol nobody ships are the same
//! question asked twice — *that is not one of them; which one did you mean?* —
//! and the answer to both is an edit distance over a fixed vocabulary. One
//! implementation rather than two, because a suggestion that ranked differently
//! in the two places would be two behaviours nobody chose.
//!
//! It is only ever run over a list this build carries, never over anything a
//! document supplies as a set, so its cost is bounded by what the binary ships.

/// Levenshtein distance: how many single-character edits turn one string into
/// the other.
///
/// Two rows rather than a full matrix, because the names involved are short and
/// this runs once per keyframe track or once per icon asset.
pub(crate) fn between(from: &str, to: &str) -> usize {
    let (from, to): (Vec<char>, Vec<char>) = (from.chars().collect(), to.chars().collect());
    let mut previous: Vec<usize> = (0..=to.len()).collect();
    let mut current = vec![0; to.len() + 1];

    for (i, from_char) in from.iter().enumerate() {
        current[0] = i + 1;
        for (j, to_char) in to.iter().enumerate() {
            let substitution = previous[j] + usize::from(from_char != to_char);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[to.len()]
}
