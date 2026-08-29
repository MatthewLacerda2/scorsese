//! Breaking a string into the lines that fit on the raster.
//!
//! Word wrapping and truncation, and deliberately nothing beyond them. Full
//! text layout — bidirectional runs, complex scripts, locale-aware line
//! breaking — is a large, stateful problem, and a title card does not have it.
//! What a title card *does* have is a string too long for the frame, which
//! without wrapping runs off the side of the picture and without truncation
//! runs off the bottom of it.

use super::font::Faces;
use super::runs;
use super::shape::{NBSP, Shaped};

/// One line, laid out and shaped.
pub(super) struct Line {
    /// The characters on it, with the whitespace it was broken at removed.
    /// Kept beside the glyphs because breaking, truncating and ellipsising all
    /// happen to text — a glyph run cannot have a character taken off its end.
    pub text: String,
    /// The glyphs that set it, and how wide they set. Shaped here rather than
    /// again at drawing time: a line is measured to decide it fits, and
    /// measuring it *is* shaping it, so the result is carried forward instead
    /// of being thrown away and recomputed for every frame.
    pub shaped: Shaped,
}

/// How wide `text` sets in one line. The one measurement in this module, so
/// wrapping and drawing can never disagree about where a line ends.
///
/// **The chain rather than a face**, and every function below takes it for the
/// same reason: a line may be set from more than one face, and a width worked
/// out from the named one alone would be the width of a line with the emoji
/// missing. Wrapping would then break where the drawn text does not.
pub(super) fn measure(faces: &Faces<'_>, text: &str) -> f32 {
    faces.shape(text).width
}

/// Wraps `text` to `max_width`, keeping at most `max_lines` of it.
///
/// Newlines in the content are honoured — an author who broke a title in two
/// meant it — and each paragraph between them is then wrapped on its own. A
/// single word too long for the line is broken between characters rather than
/// left to run off the edge: a URL or a compound word is still better read
/// broken than not read at all.
///
/// Anything past `max_lines` is dropped and the last surviving line ends in an
/// ellipsis. Truncating rather than overflowing is the honest failure: text
/// running off the bottom of the frame looks like a render bug, where an
/// ellipsis says "there is more here than fits" to whoever is watching.
pub(super) fn wrap(text: &str, faces: &Faces<'_>, max_width: f32, max_lines: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        wrap_paragraph(paragraph, faces, max_width, &mut lines);
    }
    if lines.len() <= max_lines {
        return lines;
    }
    lines.truncate(max_lines.max(1));
    if let Some(last) = lines.pop() {
        lines.push(ellipsise(last, faces, max_width));
    }
    lines
}

/// Greedy wrapping: take words until the next one would not fit, then break.
fn wrap_paragraph(paragraph: &str, faces: &Faces<'_>, max_width: f32, lines: &mut Vec<Line>) {
    let mut current = String::new();
    for word in words(paragraph) {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if measure(faces, &candidate) <= max_width {
            current = candidate;
            continue;
        }
        if !current.is_empty() {
            push(lines, faces, std::mem::take(&mut current));
        }
        // The word now starts a line of its own; if it does not fit even
        // there, it is broken between characters and the tail carries on.
        current = break_word(word, faces, max_width, lines);
    }
    // A blank line in the content is a blank line on screen, so an empty
    // paragraph still contributes one.
    if !current.is_empty() || paragraph.trim().is_empty() {
        push(lines, faces, current);
    }
}

/// The break opportunities in a paragraph: whitespace, except the one
/// character that exists in order not to be one.
///
/// `U+00A0` carries the Unicode `White_Space` property, so `split_whitespace`
/// eats it exactly as it eats a space — which undoes the whole of what a
/// non-breaking space is for. Treating it as a printing character instead is
/// what lets a line hold its own spacing: a run of them survives into the word
/// it is part of, so an indent and a column both reach the raster.
///
/// Ordinary runs still collapse, because the words are rejoined with a single
/// space above. That is the right behaviour for wrapped prose and it is not
/// what this changes.
fn words(paragraph: &str) -> impl Iterator<Item = &str> {
    paragraph
        .split(|character: char| character.is_whitespace() && character != NBSP)
        .filter(|word| !word.is_empty())
}

/// Splits a word too wide for a line, returning whatever tail still fits.
///
/// **Between clusters, not between characters.** 👍🏽 is a hand and a skin tone
/// and 👨‍👩‍👧 is three people and two joiners; each is one drawing, and a break
/// inside one would put half a picture at the end of a line and the rest at the
/// start of the next. The same rule [`super::runs`] applies to which face draws
/// what applies to where a line ends, and for the same reason.
fn break_word(word: &str, faces: &Faces<'_>, max_width: f32, lines: &mut Vec<Line>) -> String {
    let mut rest = word.to_owned();
    while measure(faces, &rest) > max_width {
        let mut head = 0;
        for cluster in runs::clusters(&rest) {
            // At least one cluster per line, whatever the width: a max_width
            // narrower than a single glyph must not loop forever.
            if head > 0 && measure(faces, &rest[..cluster.end]) > max_width {
                break;
            }
            head = cluster.end;
        }
        let taken = rest[..head].to_owned();
        rest = rest[head..].to_owned();
        push(lines, faces, taken);
    }
    rest
}

/// Replaces the tail of a line with an ellipsis, narrowing it until it fits.
fn ellipsise(last: Line, faces: &Faces<'_>, max_width: f32) -> Line {
    let mut text = last.text;
    loop {
        // Trailing whitespace goes before the ellipsis, but a non-breaking
        // space is not trailing whitespace — it is a space the author asked
        // for, and `trim_end` would take it for the same reason
        // `split_whitespace` used to.
        let candidate = format!(
            "{}…",
            text.trim_end_matches(|character: char| character.is_whitespace() && character != NBSP)
        );
        let shaped = faces.shape(&candidate);
        if shaped.width <= max_width || text.is_empty() {
            return Line {
                text: candidate,
                shaped,
            };
        }
        text.pop();
    }
}

fn push(lines: &mut Vec<Line>, faces: &Faces<'_>, text: String) {
    let shaped = faces.shape(&text);
    lines.push(Line { text, shaped });
}

#[cfg(test)]
mod tests {
    use crate::text::font::Font;

    use super::*;

    /// A word exactly as wide as its line stays on it; a hair wider does not.
    ///
    /// The boundary [`break_word`] is written on, and the mutation report is
    /// where the gap showed: the comparison survived being loosened to `>=`,
    /// because nothing had ever handed it a prefix measuring *exactly* the
    /// width it was given. Loosened, it drops the last cluster of every word
    /// that just fits — which is not a wrapping bug anybody would look at and
    /// call one, because the line still ends where a line should.
    ///
    /// Six of one letter, so that both halves measure the same and the second
    /// line's fit is the same claim as the first's.
    #[test]
    fn a_word_exactly_as_wide_as_its_line_is_broken_after_it_and_not_before() {
        let font = Font::sans();
        let faces = font.faces(20.0);
        let exact = measure(&faces, "aaa");
        let broken = |max_width: f32| {
            wrap("aaaaaa", &faces, max_width, 8)
                .into_iter()
                .map(|line| line.text)
                .collect::<Vec<_>>()
        };
        assert_eq!(broken(exact), vec!["aaa".to_owned(), "aaa".to_owned()]);
        // …and half a pixel narrower does not fit, so the equality above is
        // the thing being asserted rather than a rounding accident.
        assert_eq!(
            broken(exact - 0.5).first().map(String::as_str),
            Some("aa"),
            "one pixel over the line is over the line"
        );
    }
}
