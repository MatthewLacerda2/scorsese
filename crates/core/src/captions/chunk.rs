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
    /// The most words one caption holds; `usize::MAX` is no bound but
    /// [`Chunking::max_chars`].
    pub max_words: usize,
    /// Whether a caption also ends after a comma, semicolon or colon, not
    /// only at a sentence's end.
    pub clauses: bool,
    /// Whether a name stays in one caption: a run of capitalised words (the
    /// pronoun `I` is not one), a particle between two of them (`da` in
    /// `Lula da Silva`), and the word just before the run
    /// (`presidente Lula`). A name longer than [`Chunking::max_words`] is a
    /// caption of its own rather than cut.
    pub names: bool,
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
        max_words: usize::MAX,
        clauses: false,
        names: false,
    };

    /// The page look's cut (#1550): at most three words, broken at every
    /// punctuation mark and at a half-second pause, a name kept whole, held
    /// briefly after its last word — what read well on a 9:16 Story with one
    /// word lit at a time.
    pub const WORDS: Self = Self {
        max_chars: 36,
        lead: 0.07,
        hold: 0.35,
        bridge: 1.0,
        pause: 0.5,
        max_words: 3,
        clauses: true,
        names: true,
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
    /// The same words, each named as a page looks it up in `scorsese.words`.
    pub words: Vec<Placed>,
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
/// A caption ends after a word that ends a sentence (or, with
/// [`Chunking::clauses`], a clause), and before a word that follows a
/// [`Chunking::pause`]. When the next word would take it past
/// [`Chunking::max_chars`] or [`Chunking::max_words`], it ends there — at its
/// last comma instead, when that leaves at least two words on each side, so a
/// clause is not split from the word it was waiting for. With
/// [`Chunking::names`], a name moves as one word would.
pub fn chunks(line: &ClipId, words: &[Placed], chunking: Chunking) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut current: Vec<&Placed> = Vec::new();
    for unit in units(words, chunking) {
        let first = unit[0];
        let paused = current
            .last()
            .is_some_and(|last| first.start - last.end >= chunking.pause);
        let over = |current: &[&Placed]| {
            length(current) + 1 + length(&unit) > chunking.max_chars
                || current.len() + unit.len() > chunking.max_words
        };
        if paused {
            out.push(chunk(line, &current));
            current.clear();
        } else if !current.is_empty() && over(&current) {
            let keep = clause_end(&current).unwrap_or(current.len());
            out.push(chunk(line, &current[..keep]));
            current.drain(..keep);
        }
        current.extend(&unit);
        let last = unit[unit.len() - 1];
        if ends(last, SENTENCE) || (chunking.clauses && ends(last, CLAUSE)) {
            out.push(chunk(line, &current));
            current.clear();
        }
    }
    if !current.is_empty() {
        out.push(chunk(line, &current));
    }
    out
}

/// What ends a sentence.
const SENTENCE: &[char] = &['.', '!', '?', '…'];

/// What ends a clause.
const CLAUSE: &[char] = &[',', ';', ':'];

/// The words in the pieces a caption never splits: each word alone, or with
/// [`Chunking::names`] a name with the word before it. Nothing joins across a
/// pause or a punctuation mark.
fn units(words: &[Placed], chunking: Chunking) -> Vec<Vec<&Placed>> {
    let mut units: Vec<Vec<&Placed>> = Vec::new();
    for (i, word) in words.iter().enumerate() {
        let joins = chunking.names
            && i > 0
            && joinable(&words[i - 1], word, chunking)
            && (named(word)
                || (short(word)
                    && words
                        .get(i + 1)
                        .is_some_and(|next| named(next) && joinable(word, next, chunking))
                    && named(&words[i - 1])));
        match units.last_mut() {
            Some(unit) if joins => unit.push(word),
            _ => units.push(vec![word]),
        }
    }
    units
}

/// Whether nothing between `before` and `after` breaks a caption: no
/// punctuation after `before`, no pause before `after`.
fn joinable(before: &Placed, after: &Placed, chunking: Chunking) -> bool {
    after.start - before.end < chunking.pause && !ends(before, SENTENCE) && !ends(before, CLAUSE)
}

/// Whether `word` is written capitalised, as a name is: its first letter
/// upper case, and not the pronoun `I` (`I'm`, `I'll`).
fn named(word: &Placed) -> bool {
    let letters = word.text.trim_start_matches(|c: char| !c.is_alphanumeric());
    let pronoun = letters == "I" || letters.starts_with("I'") || letters.starts_with("I’");
    letters.chars().next().is_some_and(char::is_uppercase) && !pronoun
}

/// Whether `word` is a particle that sits inside a name between two
/// capitalised words: `da` in `Lula da Silva`, `van` in `Vincent van Gogh`.
fn short(word: &Placed) -> bool {
    PARTICLES.contains(&word.text.as_str())
}

/// The lowercase words a name carries inside it. A list, not a rule about
/// short words: `em` in `Lula em Brasília` is a preposition between two
/// names, and the two are not one.
const PARTICLES: &[&str] = &[
    "al", "bin", "da", "das", "de", "del", "della", "der", "di", "do", "dos", "du", "ibn", "la",
    "le", "van", "von",
];

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
        words: words.iter().map(|&word| word.clone()).collect(),
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
        .find(|&keep| ends(words[keep - 1], CLAUSE))
}

fn ends(word: &Placed, marks: &[char]) -> bool {
    word.text
        .trim_end_matches(['"', '\'', '”', '’', ')'])
        .ends_with(marks)
}

/// The characters of `words` joined by spaces.
fn length(words: &[&Placed]) -> usize {
    words
        .iter()
        .map(|word| word.text.chars().count())
        .sum::<usize>()
        + words.len().saturating_sub(1)
}
