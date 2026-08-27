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
//! This file walks the arrangement and hands each rendered note to the `mix`
//! module beside it, which owns where a note lands and what runs on the sums —
//! a chain on a track, or one on the whole piece. Those two stages are the
//! paragraph above one level up: some things belong to the sum and not to the
//! parts, which is also why a song's chain runs *before* the limiter here and
//! never after it.
//!
//! **Determinism.** Every note's seed is derived from `(song.seed, track
//! index, note ordinal)` through the same seeded integer hash everything else
//! here uses — no `rand`, no wall clock. The ordinal counts notes in
//! arrangement order, so a pattern played twice gets two different noise draws
//! (a repeated snare should not be a photocopy) while the whole piece stays
//! byte-identical across runs and processes. [`super::feel`] draws its onset,
//! velocity and timbre nudges from the same coordinates, so humanising a song
//! puts no asterisk on any of that — and neither does the note renderer
//! starting each oscillator somewhere in its cycle, which reads the same seed.

use std::collections::HashMap;

use super::feel::swung;
use super::mix::Mix;
use super::shape::{plan, shape};
use super::{Note, PatchRef, Song};
use crate::core::{self, RATE};
use crate::error::SynthError;
use crate::fx::limiter;
use crate::hash::hash3;
use crate::level::Layer;
use crate::note::NoteOpts;
use crate::patch::Patch;
use crate::stereo::Stereo;

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

/// A rendered song, and what each of its tracks put into it.
///
/// The two arrive together because the second only exists while the first is
/// being made: once the mix is summed there is no way back to the parts, and
/// re-rendering a track alone to measure it would be paying twice for a number
/// that was already in hand.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Mixdown {
    /// The finished stereo buffer — the thing that gets encoded.
    pub(crate) master: Stereo,
    /// One row per track, in track order, **post-gain**: what that instrument
    /// takes up in the mix rather than what it would sound like alone.
    ///
    /// The song's own chain, the master limiter and the fades are *not* in
    /// these numbers. Those belong to the sum, and a row that included them
    /// would be answering a question about the piece under the name of a
    /// track.
    ///
    /// **Empty for a song of fewer than two tracks**, which is the rule
    /// [`crate::level::Profile`] already holds sections to: one row under a
    /// one-line summary is the same sentence twice.
    pub(crate) tracks: Vec<Layer>,
}

/// Renders `song` to an interleaved stereo sample buffer at
/// [`crate::SAMPLE_RATE`], master-limited, and the length the song asks to be.
///
/// Interleaved — left sample first — because that is the form a WAV holds and
/// the form [`crate::level`] measures, so a caller doing anything at all with
/// raw samples already speaks it.
pub fn render_song(song: &Song, resolve: &dyn PatchResolver) -> Result<Vec<f32>, SynthError> {
    Ok(mix_song(song, resolve)?.master.interleaved())
}

/// [`render_song`], keeping what each track contributed on its way into the
/// mix — see [`Mixdown::tracks`].
pub(crate) fn mix_song(song: &Song, resolve: &dyn PatchResolver) -> Result<Mixdown, SynthError> {
    song.validate()?;
    let patches = resolve_patches(song, resolve)?;
    // How fast to play it and how many times through, worked out before a
    // sample is produced: `fit` is a property of the whole piece, and deciding
    // it per note would mean rendering the wrong notes and cutting afterwards.
    let (bpm, passes) = plan(song);
    // Read once: every degree in the song resolves against it, and so does
    // every diatonic lift in the arrangement.
    let key = song.key()?;
    // Every written form becomes ordinary notes here — a chord its voices, a
    // step string its hits, a degree its pitch — once per pattern and before a
    // sample is produced. So everything below this line sees notes and nothing
    // else, and each of them picks up its own ordinal, its own swing
    // displacement and its own humanise nudge, rather than a chord landing as
    // one rigid block or a hi-hat part as one photocopied strike.
    let voiced: HashMap<&str, Vec<Note>> = song
        .patterns
        .iter()
        .map(|(name, pattern)| Ok((name.as_str(), pattern.voices(key.as_ref())?)))
        .collect::<Result<_, SynthError>>()?;
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
    let mut mix = Mix::new(song, arrangement_end);
    let mut cursor_beats = 0.0f32;
    let mut ordinal: u64 = 0;
    // Resolved once: an absent `humanize` is one that scatters nothing, so the
    // note loop has a single path rather than a branch per note.
    let feel = song.humanize();

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
        for note in voiced.get(entry.pattern()).into_iter().flatten() {
            // A silenced track still consumes its ordinal, so muting one for
            // eight bars does not re-roll the noise of every note after it.
            let track = track_index[note.track.as_str()];
            let place = ordinal;
            let seed = note_seed(song.seed, track, place);
            ordinal += 1;
            if !entry.plays(&note.track) {
                continue;
            }
            // The entry's transforms, applied to the *written* pattern rather
            // than to whatever the previous entry produced: they do not stack
            // across entries, so the tenth repeat is not nine octaves up and
            // the document still says what it does.
            // The level first, then how far this strike's tone sits from it:
            // `timbre` is a fraction of the velocity actually played, so the
            // two are drawn in that order rather than independently.
            let velocity = feel.velocity(note.vel * entry.vel_scale(), track, place, song.seed);
            let opts = NoteOpts {
                duration: note.dur * beat,
                velocity,
                timbre: feel.timbre(velocity, track, place, song.seed),
                seed,
            };
            // Both transposes, applied in one place — and clamped rather than
            // refused, since refusing would make a legal transpose depend on
            // the register of a pattern written months ago.
            let pitch = entry.played_pitch(note.note.to_midi()?, key.as_ref());
            let rendered = core::render_note(&patches[track], pitch, &opts)?;
            // Swing first, humanise second: swing is where the beat *is*, and
            // humanise is how well the player hit it. Clamped at zero rather
            // than wrapped — a note nudged early on the very first beat has
            // nowhere to go, and a negative sample index is not a time.
            let onset = (cursor_beats + swung(note.start, song.swing)) * beat
                + feel.onset_seconds(track, place, song.seed);
            let at = (onset * RATE).round().max(0.0) as usize;
            // Added to the track's own bus rather than straight to the master:
            // where a note lands is timing, which bus it lands on is routing,
            // and the two answer to different fields.
            mix.add(track, &rendered, at);
        }
        cursor_beats += pattern.beats;
    }

    // Track buses folded down and the song's own chain applied — everything
    // that belongs to a sum rather than to a note. Each track is measured as it
    // goes past, which is the only moment it exists on its own.
    let (mut master, tracks) = mix.finish();
    // The master limiter, always — mixing by addition is exactly the operation
    // that overshoots full scale, so the sum is never handed out unlimited.
    limiter::apply(&mut master, RATE);
    // Then length and level, in that order, on the limited signal.
    shape(song, &mut master, arrangement_end);
    Ok(Mixdown { master, tracks })
}

/// One seed per note, from `(song seed, track, ordinal)`.
///
/// Two 32-bit hashes on different channels are stitched into the `u64` the note
/// renderer wants, so the full seed space is used rather than the low 32 bits.
///
/// Those are channels **0 and 1** of these coordinates; [`super::feel`] draws
/// its onset, velocity and timbre nudges from 2, 3 and 4 of the same pair,
/// which is what keeps a note's timing, its loudness, its tone and its noise
/// from moving together.
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
