//! The song document: patterns, arrangement, tracks.
//!
//! Where [`crate::patch`] describes *one instrument*, a [`Song`] describes *a
//! piece of music*: which instruments play (tracks), what they play (patterns
//! of notes), and in what order (the arrangement).
//!
//! The shape is **tracker-style** (the MOD/XM lineage), not a linear piano
//! roll, and deliberately so: a melody with any repetition in it is a handful
//! of short patterns and a list naming them, which is a few dozen lines an
//! agent can write, diff and iterate on. The same music as a flat note list
//! would be hundreds of lines with the repetition spelled out, and every edit
//! would be a merge conflict with itself.
//!
//! Everything is **beats**, never seconds — `bpm` converts once at render time
//! — so changing the tempo of a finished song is one number.
//!
//! ```jsonc
//! {
//!   "bpm": 120,
//!   "seed": 7,
//!   "tracks": [{ "name": "bass", "patch": "recipes/bass.json", "gain": 0.8 }],
//!   "patterns": {
//!     "verse": { "beats": 8, "notes": [
//!       { "track": "bass", "note": "E2", "start": 0.0, "dur": 0.5, "vel": 1.0 }] }
//!   },
//!   "arrangement": ["verse", { "pattern": "verse", "transpose": 12 }]
//! }
//! ```
//!
//! An arrangement entry is a pattern's name, or that name with transforms —
//! see [`arrangement`] for why a repeat that can vary is the difference between
//! music that develops and music that only repeats.
//!
//! Rendering lives in [`render`]; this file is the document alone — plain
//! serde data that round-trips losslessly, the same "document as truth" rule
//! the patch follows.

pub mod arrangement;
pub mod render;
pub mod shape;
pub mod timing;
mod validate;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::SynthError;
use crate::patch::Patch;

pub use arrangement::{ArrangementEntry, Play};
pub use render::{InlineOnly, PatchResolver, render_song};
pub use timing::{Fade, Fit, FitMode, Tail};

/// Default for a per-track or per-note gain: unity, i.e. "as written".
fn one() -> f32 {
    1.0
}

/// A complete piece of music, renderable to one mono buffer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Song {
    /// Tempo in beats per minute; the one place beats become seconds.
    pub bpm: f32,
    /// Folded into every note's render seed, so one number re-rolls every
    /// stochastic source in the piece while keeping it reproducible.
    #[serde(default)]
    pub seed: u64,
    /// The instruments, in a fixed order — the order is part of the seed
    /// derivation, so it is data, not presentation.
    pub tracks: Vec<Track>,
    /// Named blocks of notes. A `BTreeMap` rather than a `HashMap` so
    /// [`Song::to_json`] emits patterns in a stable order and two saves of the
    /// same song are byte-identical.
    pub patterns: BTreeMap<String, Pattern>,
    /// Which patterns play, in order. An entry is a pattern's name, or that
    /// name with [transforms](ArrangementEntry) — a repeat that varies rather
    /// than photocopies.
    pub arrangement: Vec<ArrangementEntry>,
    /// A length the piece has to come out at, when the picture decides that
    /// rather than the music. Absent means the song is as long as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit: Option<Fit>,
    /// Level moves on the finished piece. Absent means neither end moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade: Option<Fade>,
    /// What happens after the last beat. Absent means [`Tail::Ring`], which is
    /// what every song did before the field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<Tail>,
}

/// One instrument in the mix: a patch, and how loud it sits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// How notes refer to this track.
    pub name: String,
    /// The instrument it plays: inline, or named for a resolver to find.
    pub patch: PatchRef,
    /// Linear gain applied to every note this track plays. The master limiter
    /// guarantees the sum never clips, so this is a balance control, not a
    /// safety one.
    #[serde(default = "one")]
    pub gain: f32,
}

/// A track's instrument: the patch inline, or a reference for the caller to
/// resolve.
///
/// A song being iterated on carries its patches inline and stays one
/// self-contained file; a song built from a settled instrument library names
/// them and stays short.
///
/// What a reference *means* is not this crate's business — it resolves through
/// a [`PatchResolver`] the caller supplies. In scorsese that is a path under
/// the project root, checked by the same rules every other path obeys; nothing
/// here opens a file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatchRef {
    /// A name for the caller's resolver to turn into a patch.
    Named(String),
    /// The patch document itself. Boxed because it dwarfs the other variant.
    Inline(Box<Patch>),
}

/// A named block of notes, `beats` long.
///
/// Patterns are just N beats — no time signature, because nothing in the
/// renderer needs bars.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    /// How long this block occupies in the arrangement, in beats. Notes may
    /// ring out past it — the next pattern starts on time regardless — so this
    /// is the *slot*, not the sound.
    pub beats: f32,
    /// What is played, and when within the slot.
    pub notes: Vec<Note>,
}

/// One note: which track plays it, what pitch, when, for how long, how hard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// The [`Track::name`] that plays it.
    pub track: String,
    /// What pitch to play.
    pub note: Pitch,
    /// Onset in beats from the start of its pattern.
    pub start: f32,
    /// Gate length in beats. The patch's release rings out after it.
    pub dur: f32,
    /// Velocity in `0..=1`, scaling this note's peak amplitude.
    #[serde(default = "one")]
    pub vel: f32,
}

/// A pitch, written the way a score does or as a raw MIDI number.
///
/// Untagged, and `Name` first: a JSON string can only be a name and a JSON
/// number can only be a MIDI value, so the two never race.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Pitch {
    /// Scientific pitch notation — `"C#4"`, `"Bb3"` (see
    /// [`parse_note`](crate::parse_note)).
    Name(String),
    /// A MIDI note number; fractional values are legal microtonal pitches.
    Midi(f32),
}

impl Pitch {
    /// Resolves to a MIDI number, reporting an unparseable name.
    pub fn to_midi(&self) -> Result<f32, SynthError> {
        match self {
            Pitch::Name(name) => crate::note::parse_note(name),
            Pitch::Midi(midi) => Ok(*midi),
        }
    }
}

impl Song {
    /// Serialises to canonical JSON — pretty, with patterns in name order.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parses a song from its JSON form.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Total arrangement length in beats — the sum of its patterns' slots.
    ///
    /// Notes are allowed to ring out past this: it is where the *last pattern*
    /// ends, not where the audio does.
    pub fn arrangement_beats(&self) -> f32 {
        self.arrangement
            .iter()
            .filter_map(|entry| self.patterns.get(entry.pattern()))
            .map(|pattern| pattern.beats)
            .sum()
    }

    /// Seconds per beat.
    pub fn beat_seconds(&self) -> f32 {
        60.0 / self.bpm
    }

    /// What this song does after its last beat, defaults included — so a
    /// caller never has to decide what an absent field meant.
    pub fn tail(&self) -> Tail {
        self.tail.unwrap_or_default()
    }
}
