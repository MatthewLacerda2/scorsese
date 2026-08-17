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

/// Two strings, one number.
///
/// Which of the three edits a distance is made of — substitute, insert, delete —
/// is the whole of what the two rows compute, and each has its own term in the
/// `min` above, so each needs a pair that isolates it: a distance asserted only
/// on names differing by a substitution is satisfied by arithmetic that cannot
/// count an insertion at all. The numbers are the definition's rather than this
/// implementation's — `kitten` to `sitting` is the textbook three.
#[cfg(test)]
mod tests {
    use super::between;

    #[test]
    fn nothing_to_change_is_no_distance() {
        assert_eq!(between("", ""), 0);
        assert_eq!(between("clapperboard", "clapperboard"), 0);
    }

    /// An empty side is as far away as the other one is long, both ways round —
    /// which is what says the first row and the first column are seeded, rather
    /// than the answer falling out of the loop by luck.
    #[test]
    fn an_empty_name_is_as_far_as_the_other_is_long() {
        assert_eq!(between("arrow", ""), 5, "five deletions");
        assert_eq!(between("", "arrow"), 5, "or five insertions");
    }

    /// One of each kind, on its own, so no term of the `min` can be borrowed to
    /// cover another.
    #[test]
    fn one_edit_of_each_kind_is_one_apart() {
        assert_eq!(between("clapboard", "clipboard"), 1, "one substitution");
        assert_eq!(between("arow", "arrow"), 1, "one insertion");
        assert_eq!(between("arrrow", "arrow"), 1, "one deletion");
    }

    /// A transposed pair is two edits and not one, which is the boundary
    /// `icon::nearest` puts its ceiling at.
    #[test]
    fn a_transposed_pair_is_two() {
        assert_eq!(between("clapperbaord", "clapperboard"), 2);
    }

    /// Several at once, and mixed: the textbook three, and the three that
    /// `nearest`'s doc names as the reason its ceiling is where it is.
    #[test]
    fn a_mixture_of_edits_adds_up() {
        assert_eq!(between("kitten", "sitting"), 3);
        assert_eq!(between("clapboard", "clapperboard"), 3);
        assert_eq!(between("flaw", "lawn"), 2);
    }

    /// Symmetric, because a suggestion that depended on which of the two names
    /// was written down first would rank differently in the two places that ask
    /// for one.
    #[test]
    fn it_reads_the_same_in_either_direction() {
        for (from, to) in [("kitten", "sitting"), ("clapper", "clapperboard")] {
            assert_eq!(between(from, to), between(to, from), "{from} and {to}");
        }
    }

    /// Characters rather than bytes: `é` is two bytes and one edit.
    #[test]
    fn it_counts_characters_and_not_bytes() {
        assert_eq!(between("cafe", "café"), 1);
    }
}
