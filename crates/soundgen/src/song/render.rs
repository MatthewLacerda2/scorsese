//! Walk the arrangement, sum the notes, limit the result.
//!
//! The whole renderer is one idea: **mixing is addition**. Every note is
//! rendered independently through the note renderer, and its buffer is added
//! into the master at its start offset. There is no streaming, no voice
//! allocation and no voice-stealing, because none of that buys anything here —
//! this is not a real-time synth, it is a buffer being built, and memory is
//! cheap.
//!
//! Two deliberate consequences:
//!
//! - **Nothing is cut off.** The buffer grows to fit whatever the last note's
//!   release and fx tail need, so a song ends by ringing out rather than by a
//!   hard stop at the final beat.
//! - **Notes are rendered *unlimited*** and only the finished mix is limited.
//!   The per-note limiter exists so a single baked one-shot cannot clip its own
//!   file; applying it here as well would squash each note's dynamics *before*
//!   the mix, then squash the sum again. The master limiter is the guarantee
//!   that matters, and it is not optional.
//!
//! **Determinism.** Every note's seed is derived from `(song.seed, track
//! index, note ordinal)` through the same seeded integer hash everything else
//! here uses — no `rand`, no wall clock. The ordinal counts notes in
//! arrangement order, so a pattern played twice gets two different noise draws
//! (a repeated snare should not be a photocopy) while the whole piece stays
//! byte-identical across runs and processes.

use std::collections::HashMap;

use super::shape::{plan, shape};
use super::{PatchRef, Song};
use crate::core::{self, RATE};
use crate::error::SynthError;
use crate::fx::limiter;
use crate::hash::hash3;
use crate::note::{MIDI_RANGE, NoteOpts};
use crate::patch::Patch;

/// Supplies the patch behind a track that names its instrument rather than
/// carrying it inline.
///
/// A trait rather than a path, because this crate does no I/O and has never
/// heard of a project. The caller decides what a name means and whether it is
/// allowed to be read — in scorsese, that a recipe stays inside the project
/// root.
///
/// The failure is text because only the caller knows what went wrong; it is
/// wrapped in [`SynthError::UnresolvedPatch`] with the track and reference that
/// asked for it.
pub trait PatchResolver {
    /// Produces the patch this reference names, or says why it cannot.
    fn resolve(&self, reference: &str) -> Result<Patch, String>;
}

impl<F> PatchResolver for F
where
    F: Fn(&str) -> Result<Patch, String>,
{
    fn resolve(&self, reference: &str) -> Result<Patch, String> {
        self(reference)
    }
}

/// A resolver that refuses every reference — for songs that carry all their
/// instruments inline, where being asked to resolve one at all is the bug.
#[derive(Debug, Clone, Copy, Default)]
pub struct InlineOnly;

impl PatchResolver for InlineOnly {
    fn resolve(&self, _reference: &str) -> Result<Patch, String> {
        Err("this song must carry its patches inline".to_owned())
    }
}

/// Renders `song` to a mono sample buffer at [`crate::SAMPLE_RATE`],
/// master-limited, and the length the song asks to be.
pub fn render_song(song: &Song, resolve: &dyn PatchResolver) -> Result<Vec<f32>, SynthError> {
    song.validate()?;
    let patches = resolve_patches(song, resolve)?;
    // How fast to play it and how many times through, worked out before a
    // sample is produced: `fit` is a property of the whole piece, and deciding
    // it per note would mean rendering the wrong notes and cutting afterwards.
    let (bpm, passes) = plan(song);
    let track_index: HashMap<&str, usize> = song
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| (track.name.as_str(), index))
        .collect();

    let beat = 60.0 / bpm;
    // Start at the arrangement's own length rather than growing from empty, so
    // a song whose last pattern ends on a rest keeps that rest. Truncating to
    // the final *note* instead would silently shorten the buffer and put a loop
    // point in the wrong place — the arrangement is the score, not just where
    // the samples happen to stop. Tails then extend it past this.
    let played = song.arrangement_beats() * passes as f32;
    let arrangement_end = (played * beat * RATE).round() as usize;
    let mut master = vec![0.0f32; arrangement_end];
    let mut cursor_beats = 0.0f32;
    let mut ordinal: u64 = 0;

    // Repeats are a longer arrangement, not a copied buffer: tiling rendered
    // audio would overlap each pass's ring-out onto the next pass's downbeat,
    // and would replay the same seeded noise every time round.
    for entry in song
        .arrangement
        .iter()
        .cycle()
        .take(song.arrangement.len() * passes as usize)
    {
        let pattern =
            song.patterns
                .get(entry.pattern())
                .ok_or_else(|| SynthError::UnknownPattern {
                    pattern: entry.pattern().to_owned(),
                })?;
        for note in &pattern.notes {
            // A silenced track still consumes its ordinal, so muting one for
            // eight bars does not re-roll the noise of every note after it.
            let track = track_index[note.track.as_str()];
            let seed = note_seed(song.seed, track, ordinal);
            ordinal += 1;
            if !entry.plays(&note.track) {
                continue;
            }
            // The entry's transforms, applied to the *written* pattern rather
            // than to whatever the previous entry produced: they do not stack
            // across entries, so the tenth repeat is not nine octaves up and
            // the document still says what it does.
            let opts = NoteOpts {
                duration: note.dur * beat,
                velocity: note.vel * entry.vel_scale(),
                seed,
            };
            // Clamped rather than refused: refusing would make a legal
            // transpose depend on the register of a pattern written months ago.
            let pitch =
                (note.note.to_midi()? + entry.transpose()).clamp(MIDI_RANGE.0, MIDI_RANGE.1);
            let rendered = core::render_note(&patches[track], pitch, &opts)?;
            let at = ((cursor_beats + note.start) * beat * RATE).round() as usize;
            mix_into(&mut master, &rendered, at, song.tracks[track].gain);
        }
        cursor_beats += pattern.beats;
    }

    // The master limiter, always — mixing by addition is exactly the operation
    // that overshoots full scale, so the sum is never handed out unlimited.
    limiter::apply(&mut master, RATE);
    // Then length and level, in that order, on the limited signal.
    shape(song, &mut master, arrangement_end);
    Ok(master)
}

/// Adds `src * gain` into `master` starting at sample `at`, growing `master` to
/// fit.
///
/// The growth is what lets the song ring out: the last note's release tail
/// extends the buffer past the final beat instead of being truncated to it.
fn mix_into(master: &mut Vec<f32>, src: &[f32], at: usize, gain: f32) {
    let end = at + src.len();
    if master.len() < end {
        master.resize(end, 0.0);
    }
    for (dst, sample) in master[at..end].iter_mut().zip(src) {
        *dst += sample * gain;
    }
}

/// One seed per note, from `(song seed, track, ordinal)`.
///
/// Two 32-bit hashes on different channels are stitched into the `u64` the note
/// renderer wants, so the full seed space is used rather than the low 32 bits.
fn note_seed(song_seed: u64, track: usize, ordinal: u64) -> u64 {
    let hi = u64::from(hash3(track as i64, ordinal as i64, 0, song_seed));
    let lo = u64::from(hash3(track as i64, ordinal as i64, 1, song_seed));
    (hi << 32) | lo
}

/// Resolves every track's patch once, up front.
///
/// Up front rather than per note because a song is thousands of notes over a
/// handful of instruments. Failing here also means a missing instrument is
/// reported before any rendering happens.
fn resolve_patches(song: &Song, resolve: &dyn PatchResolver) -> Result<Vec<Patch>, SynthError> {
    song.tracks
        .iter()
        .map(|track| match &track.patch {
            PatchRef::Inline(patch) => Ok((**patch).clone()),
            PatchRef::Named(reference) => {
                resolve
                    .resolve(reference)
                    .map_err(|reason| SynthError::UnresolvedPatch {
                        track: track.name.clone(),
                        reference: reference.clone(),
                        reason,
                    })
            }
        })
        .collect()
}
