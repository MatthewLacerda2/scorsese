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
//!   "arrangement": ["verse", "verse"]
//! }
//! ```
//!
//! Rendering lives in [`render`]; this file is the document alone — plain
//! serde data that round-trips losslessly, the same "document as truth" rule
//! the patch follows.

pub mod render;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::SynthError;
use crate::patch::Patch;

pub use render::{InlineOnly, PatchResolver, render_song};

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
    /// Which patterns play, in order. Repeats are just repeats.
    pub arrangement: Vec<String>,
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
            .filter_map(|name| self.patterns.get(name))
            .map(|pattern| pattern.beats)
            .sum()
    }

    /// Seconds per beat.
    pub fn beat_seconds(&self) -> f32 {
        60.0 / self.bpm
    }

    /// Rejects a song the renderer cannot honour, *before* any samples are
    /// produced — so a typo'd track or pattern name is a clear message rather
    /// than silence in the mix, which is the failure mode that would cost an
    /// agent a whole iteration to even notice.
    pub fn validate(&self) -> Result<(), SynthError> {
        if !(self.bpm.is_finite() && self.bpm > 0.0) {
            return Err(SynthError::BadBpm { bpm: self.bpm });
        }
        if self.tracks.is_empty() {
            return Err(SynthError::NoTracks);
        }
        if self.arrangement.is_empty() {
            return Err(SynthError::EmptyArrangement);
        }
        for name in &self.arrangement {
            if !self.patterns.contains_key(name) {
                return Err(SynthError::UnknownPattern {
                    pattern: name.clone(),
                });
            }
        }
        for (name, pattern) in &self.patterns {
            pattern.validate(name, &self.tracks)?;
        }
        Ok(())
    }
}

impl Pattern {
    /// Checks one pattern's slot length and every note in it.
    fn validate(&self, name: &str, tracks: &[Track]) -> Result<(), SynthError> {
        if !(self.beats.is_finite() && self.beats > 0.0) {
            return Err(SynthError::BadPatternBeats {
                pattern: name.to_owned(),
                beats: self.beats,
            });
        }
        for (index, note) in self.notes.iter().enumerate() {
            note.validate(name, index, tracks)?;
        }
        Ok(())
    }
}

impl Note {
    /// Checks that one note names a real track, starts somewhere, lasts for
    /// some time, and has a pitch that parses.
    fn validate(&self, pattern: &str, index: usize, tracks: &[Track]) -> Result<(), SynthError> {
        let pattern = || pattern.to_owned();
        if !tracks.iter().any(|track| track.name == self.track) {
            return Err(SynthError::UnknownTrack {
                pattern: pattern(),
                index,
                track: self.track.clone(),
            });
        }
        if !(self.start.is_finite() && self.start >= 0.0) {
            return Err(SynthError::BadNoteStart {
                pattern: pattern(),
                index,
                start: self.start,
            });
        }
        if !(self.dur.is_finite() && self.dur > 0.0) {
            return Err(SynthError::BadNoteDuration {
                pattern: pattern(),
                index,
                dur: self.dur,
            });
        }
        self.note.to_midi()?;
        Ok(())
    }
}
