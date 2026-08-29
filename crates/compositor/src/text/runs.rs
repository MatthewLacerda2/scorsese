//! Which face in the chain draws which stretch of a line.
//!
//! One face covering everything is not a thing that exists, so a line is not
//! one run against one face. It is split into stretches, each set by the first
//! face in the chain that can say it, and the stretches are drawn end to end in
//! the order they were written.
//!
//! **A boundary falls between characters and never inside a cluster.** That is
//! the whole difficulty, and it is not a nicety: 👍🏽 is a thumb followed by a
//! skin-tone modifier and 👨‍👩‍👧 is three people joined by zero-width joiners, and
//! both are *ligatures* the emoji face's own `GSUB` resolves. Split between
//! their characters and each half is shaped alone, which draws a thumb followed
//! by a bare colour swatch — output that looks like a rendering glitch rather
//! than like a bug, and that no error mentions. So a cluster is chosen for as a
//! unit: whichever face can set all of it, sets all of it.
//!
//! There is no Unicode segmentation crate behind this and there does not need
//! to be. What decides a boundary here is a short list of characters that
//! *continue* what came before them — joiners, modifiers, variation selectors,
//! combining marks — and that list is the same one a shaper's clusters are
//! built from for the sequences a caption actually contains.

use std::ops::Range;

/// A stretch of a line and the face that sets it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Run {
    /// Which face in the chain, by position: `0` is the face the document
    /// named, and anything above it is a fallback.
    pub face: usize,
    /// Where the stretch sits in the line, as byte offsets.
    pub range: Range<usize>,
}

/// Splits `text` into runs, asking `covers(face, character)` which faces can
/// say what.
///
/// `faces` is how many there are to try. A cluster nothing covers is given to
/// face `0`, which is what keeps the old behaviour exactly: it shapes to
/// `.notdef`, [`super::shape`] drops it with its advance, and
/// [`super::font::Font::uncovered`] is what says so.
pub(super) fn split(text: &str, faces: usize, covers: impl Fn(usize, char) -> bool) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for cluster in clusters(text) {
        let face = choose(&text[cluster.clone()], faces, &covers);
        // Neighbouring clusters on the same face are one run, so a word is
        // shaped as a word: kerning is a per-pair feature, and a line split
        // into one run per character would lose every pair in it.
        match runs.last_mut() {
            Some(last) if last.face == face => last.range.end = cluster.end,
            _ => runs.push(Run {
                face,
                range: cluster,
            }),
        }
    }
    runs
}

/// The first face that can set the whole of `cluster`, falling back to the
/// first that can set its base character, and to face `0` when none can.
///
/// Two questions rather than one, because a cluster can be mixed: `1️⃣` is a
/// digit every text face has followed by an enclosing keycap almost none does.
/// Asking only about the base would hand that to the text face and lose the
/// keycap; asking only about the whole would find no face at all and lose both.
fn choose(cluster: &str, faces: usize, covers: &impl Fn(usize, char) -> bool) -> usize {
    let significant = || cluster.chars().filter(|c| !is_ignorable(*c));
    let whole = (0..faces).find(|face| significant().all(|c| covers(*face, c)));
    whole
        .or_else(|| {
            let base = significant().next()?;
            (0..faces).find(|face| covers(*face, base))
        })
        .unwrap_or(0)
}

/// The clusters of `text`, as byte ranges, in order and covering all of it.
///
/// Published beyond run splitting because a **line** break is under the same
/// rule as a run boundary: [`super::layout`] breaks a word too wide for its
/// line, and breaking one between a hand and its skin tone would draw the two
/// halves on separate lines.
pub(super) fn clusters(text: &str) -> Vec<Range<usize>> {
    let mut found: Vec<Range<usize>> = Vec::new();
    let mut previous = '\0';
    // How many regional indicators the cluster in progress already holds.
    // Counted rather than merely detected, because **a flag is exactly two of
    // them**: `🇧🇷🇵🇹` is Brazil and Portugal, and a rule that only asked
    // whether the neighbour was one as well would run all four into a single
    // cluster that no face has a glyph for and no line may break inside.
    let mut regionals = 0usize;
    for (at, character) in text.char_indices() {
        let end = at + character.len_utf8();
        let continues = if is_regional(character) {
            is_regional(previous) && regionals % 2 == 1
        } else {
            extends(previous, character)
        };
        match found.last_mut() {
            Some(last) if continues => {
                last.end = end;
                // Anything else in the cluster ends the pair: a flag's two
                // letters are adjacent or they are not a flag.
                regionals = if is_regional(character) {
                    regionals + 1
                } else {
                    0
                };
            }
            _ => {
                found.push(at..end);
                regionals = usize::from(is_regional(character));
            }
        }
        previous = character;
    }
    found
}

/// Whether `character` continues the cluster `previous` ended.
///
/// Everything here is a character that has no standing of its own — it modifies,
/// joins or selects a presentation for what it follows — so putting a run
/// boundary in front of one would separate a mark from the thing it marks.
///
/// Regional indicators are **not** here, and that is the reason this takes only
/// the character before it. They pair, and pairing is a fact about how many
/// have been seen rather than about the neighbour — so [`clusters`] counts
/// them and this stays stateless.
fn extends(previous: char, character: char) -> bool {
    // After a joiner comes the thing being joined, whatever it is: that is the
    // one rule that makes 👨‍👩‍👧 a single cluster rather than three.
    previous == ZWJ
        || character == ZWJ
        || is_ignorable(character)
        || is_combining(character)
        || matches!(character, '\u{1f3fb}'..='\u{1f3ff}')
}

/// Zero-width joiner: the character that makes several emoji into one.
const ZWJ: char = '\u{200d}';

/// Characters no face is ever asked to draw, and so which no face's coverage
/// decides anything about.
///
/// A variation selector chooses between a text and an emoji presentation and is
/// consumed by the shaper; a tag character spells out a subdivision flag. Noto
/// Color Emoji does not map `U+FE0F` in its `cmap` at all — it answers for it
/// in a format 14 subtable, which is a different question — so treating one as
/// a character to be covered would send every `1️⃣` to the wrong face.
pub(super) fn is_ignorable(character: char) -> bool {
    matches!(
        character,
        ZWJ | '\u{fe00}'..='\u{fe0f}' | '\u{e0020}'..='\u{e007f}' | '\u{e0100}'..='\u{e01ef}'
    )
}

/// The combining marks a caption realistically carries: accents, the enclosing
/// keycap, and the half-marks. A mark belongs to the character in front of it
/// whether or not the face that drew that character has the mark.
fn is_combining(character: char) -> bool {
    matches!(
        character,
        '\u{0300}'..='\u{036f}'
            | '\u{1ab0}'..='\u{1aff}'
            | '\u{1dc0}'..='\u{1dff}'
            | '\u{20d0}'..='\u{20f0}'
            | '\u{fe20}'..='\u{fe2f}'
    )
}

/// A regional indicator — the letters flags are spelled with, always in pairs.
fn is_regional(character: char) -> bool {
    matches!(character, '\u{1f1e6}'..='\u{1f1ff}')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chain of two: face 0 takes ASCII, face 1 takes anything at all.
    ///
    /// Face 1 covering the ASCII as well is the shape the real chain has and
    /// not a convenience — Noto Color Emoji maps the digits, because a keycap
    /// is a digit with an enclosing mark over it. A fallback that covered only
    /// what the first face lacked could never set one.
    fn ascii_then_anything(face: usize, character: char) -> bool {
        face > 0 || character.is_ascii()
    }

    fn faced(text: &str) -> Vec<(usize, &str)> {
        split(text, 2, ascii_then_anything)
            .into_iter()
            .map(|run| (run.face, &text[run.range]))
            .collect()
    }

    #[test]
    fn a_line_one_face_covers_is_one_run() {
        assert_eq!(faced("Ship it"), vec![(0, "Ship it")]);
    }

    #[test]
    fn the_stretch_the_first_face_lacks_goes_to_the_second() {
        assert_eq!(faced("Ship it 🔥"), vec![(0, "Ship it "), (1, "🔥")]);
    }

    #[test]
    fn a_skin_tone_stays_with_the_hand_it_modifies() {
        assert_eq!(faced("👍🏽"), vec![(1, "👍🏽")]);
    }

    #[test]
    fn a_joined_family_is_one_run() {
        assert_eq!(
            faced("👨\u{200d}👩\u{200d}👧"),
            vec![(1, "👨\u{200d}👩\u{200d}👧")]
        );
    }

    #[test]
    fn a_keycap_follows_its_enclosing_mark_rather_than_its_digit() {
        // `1` is ASCII, so face 0 could set it — and face 0 has no keycap, so
        // the only face that can set the whole cluster is face 1, and the
        // whole cluster is what is chosen for.
        assert_eq!(faced("1\u{fe0f}\u{20e3}"), vec![(1, "1\u{fe0f}\u{20e3}")]);
    }

    #[test]
    fn a_flag_is_one_run_and_not_two_letters() {
        assert_eq!(faced("🇧🇷"), vec![(1, "🇧🇷")]);
    }

    #[test]
    fn a_cluster_no_face_covers_falls_to_the_first() {
        // Neither face claims it, and face 0 is where it lands: shaped to
        // `.notdef`, dropped, and reported by `uncovered` rather than drawn.
        assert_eq!(
            split("x", 2, |_, _| false),
            vec![Run {
                face: 0,
                range: 0..1
            }]
        );
    }

    /// The cluster rules asserted on [`clusters`] directly, because
    /// [`split`] cannot see them.
    ///
    /// A run is a *maximal* stretch on one face, so neighbouring clusters that
    /// chose the same face are merged into one run — and an emoji sequence
    /// chooses the same face all the way through by construction. So every
    /// assertion above about a sequence being one run passes whether the
    /// clustering worked or not, and the mutation report is where that showed:
    /// `is_regional` returning `false` and the joiner arm of [`extends`] both
    /// survived every test in this file.
    ///
    /// It is not a cosmetic gap. Clustering is what [`super::layout`] breaks a
    /// line on, so a rule that quietly stopped working would put a flag's two
    /// halves — or a family and its joiner — on separate lines.
    #[test]
    fn a_flag_is_one_cluster_and_two_flags_are_two() {
        // Regional indicators pair up, and only in pairs: four of them are two
        // flags rather than one long one.
        assert_eq!(clusters("🇧🇷").len(), 1);
        assert_eq!(clusters("🇧🇷🇵🇹").len(), 2);
        // And a lone one is a cluster of its own rather than joining the
        // letter beside it.
        assert_eq!(clusters("a🇧").len(), 2);
    }

    #[test]
    fn a_joiner_takes_what_follows_it_into_the_cluster() {
        // `👩` is neither ignorable nor combining nor a modifier, so the only
        // reason it belongs with what came before is that a joiner preceded
        // it. Three people joined are one cluster; the same three unjoined are
        // three.
        assert_eq!(clusters("👨\u{200d}👩\u{200d}👧").len(), 1);
        assert_eq!(clusters("👨👩👧").len(), 3);
    }

    #[test]
    fn a_skin_tone_and_a_mark_extend_the_cluster_they_follow() {
        assert_eq!(clusters("👍🏽").len(), 1);
        assert_eq!(clusters("e\u{301}").len(), 1);
        assert_eq!(clusters("1\u{fe0f}\u{20e3}").len(), 1);
        // Ordinary characters do not: two letters are two clusters, so the
        // rules above are additions to a per-character default rather than a
        // clustering that swallows everything.
        assert_eq!(clusters("ab").len(), 2);
    }

    #[test]
    fn runs_cover_the_line_in_order_with_no_gaps() {
        let text = "a🔥b🔥c";
        let runs = split(text, 2, ascii_then_anything);
        let mut at = 0;
        for run in &runs {
            assert_eq!(run.range.start, at);
            at = run.range.end;
        }
        assert_eq!(at, text.len());
        assert_eq!(runs.len(), 5);
    }
}
