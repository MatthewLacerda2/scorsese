//! The one "did you mean …?" rule (#1522).
//!
//! A misspelt argument and a misspelt property path both answer with the name
//! that was probably meant, and both keep the same rule: one place says it, so
//! the two cannot drift apart.

/// The candidate `name` was probably a typo of, when exactly one stands out.
///
/// A candidate is close when it is at most two edits from `name` (a swapped
/// pair of letters costs exactly two) and those edits are under half its
/// length, so a short word is never "close" to an unrelated one. A candidate
/// failing either test is dropped before the closest are taken. Nothing is
/// suggested unless exactly one candidate is the closest: a wrong suggestion
/// is worse than none, because it is the one thing a reader acts on without
/// checking. A candidate listed twice is still one candidate.
pub fn closest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    /// Two edits: a swapped pair of letters costs exactly this.
    const NEAREST: usize = 2;

    let mut best: Option<(usize, &str)> = None;
    let mut tied = false;
    for candidate in candidates {
        let apart = strsim::levenshtein(name, candidate);
        if apart > NEAREST || apart * 2 >= candidate.len() {
            continue;
        }
        match best {
            Some((closest, _)) if apart > closest => {}
            Some((closest, kept)) if apart == closest => tied |= kept != candidate,
            _ => {
                best = Some((apart, candidate));
                tied = false;
            }
        }
    }
    best.filter(|_| !tied).map(|(_, candidate)| candidate)
}

#[cfg(test)]
mod tests {
    use super::closest;

    #[test]
    fn a_transposition_is_caught() {
        assert_eq!(
            closest("opactiy", ["opacity", "scale", "rotation"]),
            Some("opacity")
        );
    }

    #[test]
    fn three_edits_is_too_far() {
        assert_eq!(closest("opxxxty", ["opacity"]), None);
    }

    #[test]
    fn edits_must_be_under_half_the_candidates_length() {
        // One edit from `x`, but half of a two-letter word.
        assert_eq!(closest("xy", ["x"]), None);
        assert_eq!(closest("xy", ["ab"]), None);
    }

    #[test]
    fn two_equally_close_say_nothing() {
        assert_eq!(closest("cat", ["bat", "hat"]), None);
    }

    #[test]
    fn the_same_candidate_twice_is_one() {
        assert_eq!(closest("opactiy", ["opacity", "opacity"]), Some("opacity"));
    }

    /// The tie the two old copies answered differently: `ab` is as close to
    /// `abcd` as `abcdef` is, but two edits are not under half of `ab`'s
    /// length, so it is no candidate and `abcdef` stands alone.
    #[test]
    fn a_short_far_candidate_does_not_silence_a_close_one() {
        assert_eq!(closest("abcd", ["ab", "abcdef"]), Some("abcdef"));
        assert_eq!(closest("abcd", ["abcdef", "ab"]), Some("abcdef"));
    }

    #[test]
    fn an_exact_match_is_its_own_answer() {
        assert_eq!(closest("scale", ["scale", "scales"]), Some("scale"));
        assert_eq!(closest("ab", ["ab"]), Some("ab"));
    }

    #[test]
    fn one_edit_in_three_letters_is_close() {
        assert_eq!(closest("fot", ["fit"]), Some("fit"));
    }

    #[test]
    fn a_closer_candidate_listed_later_wins() {
        assert_eq!(closest("scale", ["scales", "scale"]), Some("scale"));
    }

    #[test]
    fn nothing_to_choose_from_says_nothing() {
        assert_eq!(closest("scale", []), None);
    }
}
