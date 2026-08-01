//! Where the notes are summed, and where an effect that belongs to the *mix*
//! runs rather than to one instrument.
//!
//! [`crate::patch::Patch`] carries an fx chain too, and it means something
//! different: a patch's chain is the **instrument's** — a gunshot's corridor —
//! and it is applied to every note of that instrument independently, on its way
//! into the mix. That is the right home for the sound of an *object* and the
//! wrong home for the sound of a *room*, because a room is shared. If a piece
//! can only say "reverb" per patch, then every patch in it has to spell out the
//! same settings and keep them in sync by hand; nothing in the format suggests
//! that, so nobody does it, and the result is a set of sounds sitting next to
//! each other rather than a mix.
//!
//! So there are three places a chain can live, and the order between them is
//! the whole design:
//!
//! ```text
//!   notes ─► patch fx ─► track bus ─► track fx ─► × gain ─┐
//!                                                         ├─► song fx ─► limiter
//!   notes ─► patch fx ──────────────────────────► × gain ─┘
//! ```
//!
//! A **track** chain runs on that instrument's whole part, before its gain
//! reaches the master: it shapes an instrument. A **song** chain runs on the
//! sum: it shapes the piece. Any other ordering would blur the two into one
//! control whose meaning depended on how many tracks happened to be playing.
//!
//! Three consequences worth stating, because each is a decision rather than an
//! accident:
//!
//! - **Song fx run before the master limiter, never after.** The limiter is the
//!   guarantee that a bake cannot clip — [`super::render`] explains why it is
//!   not optional — and an effect able to add gain after it would withdraw that
//!   promise quietly.
//! - **A bus rings out.** A chain on the sum decays *once*, across everything,
//!   and past the last note, so each buffer grows by what its chain needs the
//!   same way a note's own fx tail grows the note. [`super::shape`] still cuts
//!   that to `tail` and `fit` afterwards; nothing here decides length.
//! - **A track with no chain is not bussed at all.** Its notes are added
//!   straight into the master at its gain, which is both the cheap path and the
//!   *bit-identical* one. Summing into a bus at unity and scaling once at
//!   fold-down is algebraically the same, but `(a + b)·g` and `a·g + b·g` are
//!   not the same `f32`, and a field whose whole promise is that leaving it out
//!   changes nothing cannot afford "nothing, to within rounding".

use super::Song;
use crate::core::RATE;
use crate::fx;
use crate::patch::Fx;

/// The buses a song is summed through: one master, and a private buffer for
/// each track that asked for a chain of its own.
///
/// It borrows the song rather than copying the gains and chains out of it, so
/// there is no second copy of the mix's settings to fall out of step with the
/// document — and so a caller cannot accidentally fold one song's buses down
/// against another song's tracks.
pub(super) struct Mix<'a> {
    song: &'a Song,
    master: Vec<f32>,
    /// One slot per track, in track order. `None` until that track's first
    /// note arrives, and forever if the track has no chain — the common case
    /// allocates nothing.
    buses: Vec<Option<Vec<f32>>>,
}

impl<'a> Mix<'a> {
    /// A master as long as the arrangement, and room for a bus per track.
    ///
    /// Starting at the arrangement's own length rather than growing from empty
    /// is what keeps a song that ends on a rest that long; see
    /// [`super::render`].
    pub(super) fn new(song: &'a Song, arrangement_end: usize) -> Self {
        Self {
            song,
            master: vec![0.0f32; arrangement_end],
            buses: song.tracks.iter().map(|_| None).collect(),
        }
    }

    /// Adds one rendered note of `track`, starting at sample `at`.
    ///
    /// Either straight into the master at the track's gain, or into that
    /// track's bus at unity so its chain sees the instrument's whole part
    /// before the gain is applied.
    pub(super) fn add(&mut self, track: usize, src: &[f32], at: usize) {
        let played = &self.song.tracks[track];
        if played.fx.is_empty() {
            mix_into(&mut self.master, src, at, played.gain);
        } else {
            mix_into(self.buses[track].get_or_insert_default(), src, at, 1.0);
        }
    }

    /// Folds every bus down and applies the song's own chain, handing back the
    /// summed mix **unlimited** — limiting is the renderer's last word, not the
    /// mixer's.
    pub(super) fn finish(mut self) -> Vec<f32> {
        let song = self.song;
        for (track, bus) in song.tracks.iter().zip(std::mem::take(&mut self.buses)) {
            let Some(mut bus) = bus else { continue };
            ring_out(&mut bus, &track.fx);
            fx::apply_chain(&mut bus, &track.fx, RATE);
            mix_into(&mut self.master, &bus, 0, track.gain);
        }
        ring_out(&mut self.master, &song.fx);
        fx::apply_chain(&mut self.master, &song.fx, RATE);
        self.master
    }
}

/// Grows `buf` by however long `chain` needs to decay into.
///
/// A dry chain, and no chain at all, ask for nothing — so this is a no-op on
/// the path every existing song takes.
fn ring_out(buf: &mut Vec<f32>, chain: &[Fx]) {
    let tail = (fx::tail_seconds(chain) * RATE).round() as usize;
    buf.resize(buf.len() + tail, 0.0);
}

/// Adds `src * gain` into `dst` starting at sample `at`, growing `dst` to fit.
///
/// The growth is what lets a song ring out: the last note's release tail
/// extends the buffer past the final beat instead of being truncated to it.
fn mix_into(dst: &mut Vec<f32>, src: &[f32], at: usize, gain: f32) {
    let end = at + src.len();
    if dst.len() < end {
        dst.resize(end, 0.0);
    }
    for (slot, sample) in dst[at..end].iter_mut().zip(src) {
        *slot += sample * gain;
    }
}
