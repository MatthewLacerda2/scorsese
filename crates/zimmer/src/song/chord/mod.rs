//! A chord: one entry in a pattern that sounds as several notes.
//!
//! Before this, a chord was three or four [`Note`]s with hand-computed
//! pitches, and the document said nothing about their being one thing. That is
//! **coupling with no mechanism**: transposing the chord is four coordinated
//! edits, changing its quality is four more, and any one of them can disagree
//! with the others — producing a chord that is not wrong enough to be refused
//! and not right enough to be what was meant. Nothing catches that; only ears
//! find it.
//!
//! So a chord is one entry, and it expands to notes **before** anything else
//! in the renderer touches them. Each voice is an ordinary note from that
//! point on: it gets its own swing displacement, its own humanise nudge and its
//! own render seed, which is why a chord written this way does not land as one
//! perfectly simultaneous block. That is not a bonus feature, it is what a
//! played chord sounds like, and it comes free from putting expansion first.
//!
//! ## The voicing rule, and why it is the boring one
//!
//! `Dm7` names four pitch classes and not which octave each sits in, and the
//! difference between a good voicing and a bad one is most of what a chord
//! sounds like. This module picks **close position from the root**: the root at
//! `oct`, then each remaining chord tone at the next semitone offset above it,
//! nothing dropped, nothing spread. `Dm7` at octave 3 is `D3 F3 A3 C4`.
//!
//! Three reasons, in order of how much they weigh:
//!
//! 1. **It is the voicing the name literally describes.** A reader works it out
//!    in their head, and a document whose sound cannot be predicted from
//!    reading it is not a document.
//! 2. **It re-qualifies and transposes without surprise.** `Dm7` to `D7` moves
//!    exactly one voice; a transpose moves all of them by the same amount. A
//!    cleverer default — drop-2, spread, voice-leading between neighbours —
//!    would make each of those edits move notes the writer did not ask about.
//! 3. **Anything cleverer is taste**, and taste is a property *value*. This
//!    crate defines property types and leaves values to the document, the same
//!    rule that lets a recipe choose a colour and forbids the code choosing
//!    red.
//!
//! Which leaves the escape hatch, and it has to exist because a real
//! arrangement will want a voicing no name spells. `chord` therefore takes
//! **either a name or the pitches themselves** — untagged, told apart by a JSON
//! string against a JSON array, the trick [`Pitch`] already uses. Spelling the
//! pitches keeps the one thing the name form was for: the voices are still one
//! entry, with one `start`, one `dur` and one `vel` between them.
//!
//! A `/bass` in the name covers the common inversion cheaply — see
//! [`name`] for the grammar and for what it refuses.

pub(crate) mod name;

use serde::{Deserialize, Serialize};

use super::{Note, Pitch, one};
use crate::error::SynthError;
use crate::note::MIDI_RANGE;

/// Where a chord's root sits when the document does not say.
///
/// Octave 3 puts a close-position seventh chord across middle C — the register
/// a pair of hands actually plays chords in, and low enough that a ninth on top
/// still has somewhere to go.
const DEFAULT_OCT: i32 = 3;

/// Several notes as one idea: which track plays them, what they are, when, for
/// how long, how hard.
///
/// Every field but `chord` and `oct` means exactly what it means on a
/// [`Note`], because every voice becomes one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chord {
    /// The [`Track::name`](super::Track::name) that plays it.
    pub track: String,
    /// What to play: a name, or the pitches spelled out.
    pub chord: Voicing,
    /// Which octave the **root** sits in, in scientific pitch notation, so
    /// `"Dm7"` at `3` is rooted on `D3`. Absent means octave 3, which puts a
    /// close-position seventh chord across middle C.
    ///
    /// Meaningless — and refused — beside a chord spelled as pitches, which
    /// already carries an octave per voice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oct: Option<i32>,
    /// Onset in beats from the start of its pattern.
    pub start: f32,
    /// Gate length in beats, for every voice. The patch's release rings out
    /// after it.
    pub dur: f32,
    /// Velocity in `0..=1` for every voice, before the performance scatters
    /// them apart.
    #[serde(default = "one")]
    pub vel: f32,
}

/// What a chord is: a name for the grammar to read, or the pitches themselves.
///
/// Untagged, and `Name` first: a JSON string can only be a name and a JSON
/// array can only be a list of pitches, so the two never race.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Voicing {
    /// A chord name — `"Dm7"`, `"F#maj9"`, `"C/G"`. Voiced close position from
    /// the root at [`Chord::oct`]; see the module doc for the grammar and for
    /// why that default and not a cleverer one.
    Name(String),
    /// The pitches themselves, low to high, for a voicing no name spells.
    Pitches(Vec<Pitch>),
}

impl Chord {
    /// Appends this chord's voices to `out`, or says why it cannot.
    ///
    /// Appends rather than returns, so a pattern's voices are built into one
    /// allocation and the ordinals the renderer hands out stay in document
    /// order. Nothing is appended when the chord is refused.
    pub(crate) fn voice_into(&self, out: &mut Vec<Note>) -> Result<(), SynthError> {
        match &self.chord {
            Voicing::Name(name) => {
                let spelled = name::spell(name)?;
                let root = (self.oct.unwrap_or(DEFAULT_OCT) + 1) * 12 + spelled.root;
                let range = MIDI_RANGE.0 as i32..=MIDI_RANGE.1 as i32;
                for offset in &spelled.offsets {
                    let midi = root + offset;
                    if !range.contains(&midi) {
                        return Err(SynthError::ChordOutOfRange {
                            chord: name.clone(),
                            oct: self.oct.unwrap_or(DEFAULT_OCT),
                            midi,
                        });
                    }
                    out.push(self.voice(Pitch::Midi(midi as f32)));
                }
            }
            Voicing::Pitches(pitches) => {
                if let Some(oct) = self.oct {
                    return Err(SynthError::SpelledChordOctave {
                        track: self.track.clone(),
                        start: self.start,
                        oct,
                    });
                }
                if pitches.is_empty() {
                    return Err(SynthError::EmptyChord {
                        track: self.track.clone(),
                        start: self.start,
                    });
                }
                for pitch in pitches {
                    // Checked here so a nonsense name is reported against the
                    // chord that holds it rather than at the sample loop.
                    pitch.to_midi()?;
                    out.push(self.voice(pitch.clone()));
                }
            }
        }
        Ok(())
    }

    /// One voice of this chord: the chord's timing and force, one pitch.
    fn voice(&self, note: Pitch) -> Note {
        Note {
            track: self.track.clone(),
            note,
            start: self.start,
            dur: self.dur,
            vel: self.vel,
        }
    }
}
