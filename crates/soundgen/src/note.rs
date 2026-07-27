//! Notes, pitch, and the per-note render options.
//!
//! A patch is *playable as a note*: `(pitch, velocity, duration) in → buffer out`.
//! This module owns the pitch half of that contract — the note **name ↔ MIDI ↔ Hz**
//! conversions — plus [`NoteOpts`], the small bundle carrying the other two (and the
//! seed every stochastic source draws from).
//!
//! Pitch is **equal temperament** anchored at A4 = 440 Hz (MIDI 69):
//! `f = 440 × 2^((midi − 69) / 12)`. MIDI is kept as `f32` throughout, so a
//! fractional note number is a legal microtonal pitch and so the LFO can bend pitch
//! in fractions of a semitone.
//!
//! Note names are the usual scientific pitch notation: a letter `A`–`G`, any
//! number of accidentals (`#`/`s` sharp, `b`/`f` flat), then the octave
//! (`C4` = middle C = MIDI 60, `C-1` = MIDI 0). Parsing lives here because a
//! song's notes reuse it.

use crate::error::SynthError;

/// MIDI note number of the tuning reference, A4.
const A4_MIDI: f32 = 69.0;
/// Concert pitch of A4, in Hz.
const A4_HZ: f32 = 440.0;

/// Semitone offset of each natural note within its octave (C = 0).
const NATURALS: [(char, i32); 7] = [
    ('c', 0),
    ('d', 2),
    ('e', 4),
    ('f', 5),
    ('g', 7),
    ('a', 9),
    ('b', 11),
];

/// How one note is rendered: how long the key is held, how hard it is struck, and
/// the seed the stochastic sources (noise, Karplus excitation) draw from.
///
/// `duration` is the **gate** length — the amp envelope's release rings out *after*
/// it, so the rendered buffer is longer than `duration` (see
/// [`crate::render_note`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteOpts {
    /// Gate length in seconds — how long the note is held before release.
    pub duration: f32,
    /// Striking force in `0..=1`, scaling the note's peak amplitude.
    pub velocity: f32,
    /// Seed for every stochastic source; the same seed replays the same noise.
    pub seed: u64,
}

impl Default for NoteOpts {
    /// A half-second note at a comfortable velocity, seed 0 — sane for a one-shot.
    fn default() -> Self {
        Self {
            duration: 0.5,
            velocity: 0.8,
            seed: 0,
        }
    }
}

/// Parses a note name (`"C#4"`, `"Bb3"`, `"a-1"`) to its MIDI number.
///
/// Case-insensitive; accidentals stack, so `"Cbb4"` is 58.
pub fn parse_note(name: &str) -> Result<f32, SynthError> {
    let text = name.trim().to_ascii_lowercase();
    let mut chars = text.chars();
    let letter = chars.next().ok_or(SynthError::EmptyNoteName)?;
    let semitone = NATURALS
        .iter()
        .find(|(natural, _)| *natural == letter)
        .map(|(_, semitone)| *semitone)
        .ok_or_else(|| SynthError::BadNoteLetter {
            name: name.to_owned(),
            letter,
        })?;

    let rest: String = chars.collect();
    let (accidental, octave_text) = split_accidentals(&rest);
    let octave: i32 = octave_text.parse().map_err(|_| SynthError::BadOctave {
        name: name.to_owned(),
        octave: octave_text.to_owned(),
    })?;

    let midi = (octave + 1) * 12 + semitone + accidental;
    if !(0..=127).contains(&midi) {
        return Err(SynthError::NoteOutOfRange {
            name: name.to_owned(),
            midi,
        });
    }
    Ok(midi as f32)
}

/// Splits the accidental run off the front of `rest`, returning its total
/// semitone offset and the remaining (octave) text.
fn split_accidentals(rest: &str) -> (i32, &str) {
    let mut offset = 0;
    let mut end = 0;
    for (i, c) in rest.char_indices() {
        match c {
            '#' | 's' => offset += 1,
            'b' | 'f' => offset -= 1,
            _ => break,
        }
        end = i + c.len_utf8();
    }
    (offset, &rest[end..])
}

/// Equal-temperament frequency of a (possibly fractional) MIDI note, in Hz.
#[inline]
pub fn midi_to_freq(midi: f32) -> f32 {
    A4_HZ * ((midi - A4_MIDI) / 12.0).exp2()
}
