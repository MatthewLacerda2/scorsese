//! Cutting a line's words into captions, and timing each one.
//!
//! Pure arithmetic over words already on the timeline ([`Placed`]): no
//! project, no files. That is what lets the rule be tested against a line's
//! stored timings without anything else in the room.

use crate::ClipId;
use crate::words::Placed;

/// How a line is cut and how its pieces are timed, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chunking {
    /// The most characters one caption holds, spaces included: about two
    /// lines at the caption's size. A single word longer than this is a
    /// caption of its own rather than a word cut in half.
    pub max_chars: usize,
    /// How long before its first word a caption arrives, so the eye is there
    /// when the ear is.
    pub lead: f64,
    /// How long a caption stays after its last word when nothing follows it
    /// closely.
    pub hold: f64,
    /// The longest silence a caption waits through for the next one. Closer
    /// than this, it stays until the next arrives, so the screen does not
    /// blink empty between two pieces of one thought; further, it leaves
    /// [`Chunking::hold`] after its last word.
    pub bridge: f64,
    /// A silence this long between two words breaks the caption there, with
    /// or without punctuation: the speaker paused, and so does the text.
    pub pause: f64,
}

impl Chunking {
    /// About two lines of a phone-sized caption, cut where the voice and the
    /// punctuation already cut.
    pub const DEFAULT: Self = Self {
        max_chars: 36,
        lead: 0.07,
        hold: 0.5,
        bridge: 1.0,
        pause: 0.6,
    };
}

impl Default for Chunking {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One caption: the words it shows and when, in seconds of the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// The narration clip whose words these are.
    pub line: ClipId,
    /// The words, as written, joined by single spaces.
    pub text: String,
    /// When the caption arrives: its first word, less [`Chunking::lead`].
    pub start: f64,
    /// When it leaves: where the next one arrives, or a hold after its last
    /// word.
    pub end: f64,
    /// When its last word ends — what [`time`] holds after.
    pub(crate) said_until: f64,
}

/// Cuts one line's words into captions, untimed: each one's `start` and `end`
/// are still its first word's start and last word's end.
///
/// A caption ends after a word that ends a sentence, and before a word that
/// follows a [`Chunking::pause`]. When the next word would take it past
/// [`Chunking::max_chars`], it ends there — at its last comma instead, when
/// that leaves at least two words on each side, so a clause is not split from
/// the word it was waiting for.
pub fn chunks(line: &ClipId, words: &[Placed], chunking: Chunking) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut current: Vec<&Placed> = Vec::new();
    for word in words {
        let paused = current
            .last()
            .is_some_and(|last| word.start - last.end >= chunking.pause);
        if paused {
            out.push(chunk(line, &current));
            current.clear();
        } else if !current.is_empty() && length(&current) + 1 + chars(word) > chunking.max_chars {
            let keep = clause_end(&current).unwrap_or(current.len());
            out.push(chunk(line, &current[..keep]));
            current.drain(..keep);
        }
        current.push(word);
        if ends(word, &['.', '!', '?', '…']) {
            out.push(chunk(line, &current));
            current.clear();
        }
    }
    if !current.is_empty() {
        out.push(chunk(line, &current));
    }
    out
}

/// Times every caption against every other, whichever line it came from:
/// each arrives [`Chunking::lead`] before its first word and never before the
/// timeline starts, and leaves per [`Chunking::bridge`] — never after the
/// next one arrives, so two captions never share the screen.
pub fn time(mut chunks: Vec<Chunk>, chunking: Chunking) -> Vec<Chunk> {
    chunks.sort_by(|a, b| a.start.total_cmp(&b.start));
    for chunk in &mut chunks {
        chunk.start = (chunk.start - chunking.lead).max(0.0);
    }
    let arrivals: Vec<f64> = chunks.iter().skip(1).map(|next| next.start).collect();
    for (chunk, next) in chunks
        .iter_mut()
        .zip(arrivals.iter().map(Some).chain([None]))
    {
        let held = chunk.said_until + chunking.hold;
        chunk.end = match next {
            Some(&next) if next - chunk.said_until <= chunking.bridge => next,
            Some(&next) => held.min(next),
            None => held,
        };
    }
    chunks
}

fn chunk(line: &ClipId, words: &[&Placed]) -> Chunk {
    let text = words
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let start = words.first().map_or(0.0, |word| word.start);
    let said_until = words.last().map_or(start, |word| word.end);
    Chunk {
        line: line.clone(),
        text,
        start,
        end: said_until,
        said_until,
    }
}

/// How many words to keep when breaking at the last comma, if there is one
/// with at least two words before it and two after.
fn clause_end(words: &[&Placed]) -> Option<usize> {
    (2..=words.len().saturating_sub(2))
        .rev()
        .find(|&keep| ends(words[keep - 1], &[',', ';', ':']))
}

fn ends(word: &Placed, marks: &[char]) -> bool {
    word.text
        .trim_end_matches(['"', '\'', '”', '’', ')'])
        .ends_with(marks)
}

fn chars(word: &Placed) -> usize {
    word.text.chars().count()
}

/// The characters of `words` joined by spaces.
fn length(words: &[&Placed]) -> usize {
    words.iter().map(|word| chars(word)).sum::<usize>() + words.len().saturating_sub(1)
}
