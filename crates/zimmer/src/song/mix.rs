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
//!   notes ─► patch fx ─► track bus ─► track fx ─► × gain × pan ─┐
//!                                                               ├─► song fx ─► limiter
//!   notes ─► patch fx ──────────────────────────► × gain × pan ─┘
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
//!   changes nothing cannot afford "nothing, to within rounding". `pan` keeps
//!   that promise the same way: dead centre is a literal `1.0` per side rather
//!   than the `0.99999994` the pan law computes there.
//!
//! ## Where a part is put
//!
//! **`pan` is applied at the same moment `gain` is** — on the way into the
//! master, and never before. Both are answers to "where does this instrument
//! sit", one in level and one in position, and a track's own fx chain is the
//! instrument rather than its place in the mix: a delay that ran after the pan
//! would echo from wherever the fader had put the part, which is a delay that
//! changes character when an instrument is moved.
//!
//! The two multiply into one gain per side, so a note is walked once per
//! channel and not twice. [`crate::stereo::pan_gains`] carries the law.
//!
//! ## Measuring on the way past
//!
//! The sum is also the one moment at which every track exists separately, so it
//! is where each one is measured. A report that says a mix is muddy without
//! saying which layer is muddying it sends its reader to change four
//! instruments at once and hope; [`crate::level::layer`] is what the rows are
//! for.
//!
//! It costs a buffer per track and a second add per note, and that is the
//! honest price rather than a free ride: a track with no chain is **not**
//! routed through a bus to get it, for exactly the rounding reason above. The
//! measurement takes a parallel copy instead of taking over the path, so the
//! samples handed out are bit-identical to the ones this mixer produced before
//! it measured anything. A song of one track skips it entirely — one row under
//! a one-line summary is the same sentence twice, so there is nothing to pay
//! for.

use super::Song;
use crate::core::{RATE, SAMPLE_RATE};
use crate::fx;
use crate::level::Layer;
use crate::patch::Fx;
use crate::stereo::{self, Stereo};

/// The buses a song is summed through: one master, and a private buffer for
/// each track that asked for a chain of its own.
///
/// It borrows the song rather than copying the gains and chains out of it, so
/// there is no second copy of the mix's settings to fall out of step with the
/// document — and so a caller cannot accidentally fold one song's buses down
/// against another song's tracks.
pub(super) struct Mix<'a> {
    song: &'a Song,
    master: Stereo,
    /// One slot per track, in track order: that track's own part of the mix.
    /// `None` until its first note arrives.
    ///
    /// It is the **bus** for a track that asked for a chain, and a
    /// measurement-only tap for one that did not — either way it holds what
    /// that instrument contributes and nothing else, which is what both jobs
    /// want.
    parts: Vec<Option<Stereo>>,
    /// Whether a tap is kept for the tracks that have no chain of their own.
    ///
    /// False for a song of fewer than two tracks, whose rows are not reported
    /// at all — see the module doc.
    measured: bool,
}

impl<'a> Mix<'a> {
    /// A master as long as the arrangement, and room for a part per track.
    ///
    /// Starting at the arrangement's own length rather than growing from empty
    /// is what keeps a song that ends on a rest that long; see
    /// [`super::render`].
    pub(super) fn new(song: &'a Song, arrangement_end: usize) -> Self {
        Self {
            song,
            master: Stereo::silence(arrangement_end),
            parts: song.tracks.iter().map(|_| None).collect(),
            measured: song.tracks.len() > 1,
        }
    }

    /// Adds one rendered note of `track`, starting at sample `at`.
    ///
    /// Either straight into the master at the track's gain and pan, or into
    /// that track's bus at unity and dead centre, so its chain sees the
    /// instrument's whole part before either is applied.
    pub(super) fn add(&mut self, track: usize, src: &Stereo, at: usize) {
        let played = &self.song.tracks[track];
        if !played.fx.is_empty() {
            mix_into(self.parts[track].get_or_insert_default(), src, at, UNITY);
            return;
        }
        let placed = placement(played.gain, played.pan);
        mix_into(&mut self.master, src, at, placed);
        if self.measured {
            // At gain and pan, like the master addition beside it and unlike a
            // bus: there is no chain here to see the part before the fader,
            // and the row this ends up in is what the track contributes.
            mix_into(self.parts[track].get_or_insert_default(), src, at, placed);
        }
    }

    /// Folds every bus down and applies the song's own chain, handing back the
    /// summed mix **unlimited** — limiting is the renderer's last word, not the
    /// mixer's — and one measured row per track.
    pub(super) fn finish(mut self) -> (Stereo, Vec<Layer>) {
        let song = self.song;
        let mut parts = std::mem::take(&mut self.parts);
        for (track, part) in song.tracks.iter().zip(&mut parts) {
            let Some(bus) = part.as_mut().filter(|_| !track.fx.is_empty()) else {
                continue;
            };
            ring_out(bus, &track.fx);
            fx::apply_chain(bus, &track.fx, RATE);
            let placed = placement(track.gain, track.pan);
            mix_into(&mut self.master, bus, 0, placed);
            // Scaled after the fold-down rather than before it, so the master
            // is summed from exactly the samples it always was and only the
            // copy being measured moves.
            scale(bus, placed);
        }
        ring_out(&mut self.master, &song.fx);
        fx::apply_chain(&mut self.master, &song.fx, RATE);
        let layers = if self.measured {
            measure(song, parts, self.master.frames())
        } else {
            Vec::new()
        };
        (self.master, layers)
    }
}

/// A placement that changes nothing: unity on both sides. What a track's own
/// bus is summed at, because its fader and its pan are applied later, on the
/// way to the master.
const UNITY: (f32, f32) = (1.0, 1.0);

/// The gain each side of a track's part is added at: its fader and its
/// position, multiplied into one number per channel.
///
/// One multiply rather than two passes, and the reason [`UNITY`] is a value
/// rather than a special case — everything that adds into a buffer here adds
/// at *some* placement, and a bus's happens to be the identity.
fn placement(gain: f32, pan: f32) -> (f32, f32) {
    let (l, r) = stereo::pan_gains(pan);
    (gain * l, gain * r)
}

/// One row per track, each measured over the length of the finished mix.
///
/// Padded to that length rather than measured over its own, because the rows
/// are read *against each other*: a hat that plays for four bars of a
/// forty-second piece does not take up more room than the pad under it, and a
/// mean over each track's own extent would say it did.
///
/// A track that never played still gets a row, saying it is silent. A missing
/// row reads as an oversight, and "the arp is not in this mix" is a finding.
///
/// A row is measured on **both channels**, interleaved, exactly as the mix
/// above it is. That is what keeps a hard-panned part from reading quiet: its
/// energy is all on one side, and a measurement of one side, or of a fold-down,
/// would report a number that says the instrument is missing rather than that
/// it is over there.
fn measure(song: &Song, parts: Vec<Option<Stereo>>, frames: usize) -> Vec<Layer> {
    song.tracks
        .iter()
        .zip(parts)
        .map(|(track, part)| {
            let mut part = part.unwrap_or_default();
            if !part.is_empty() {
                part.grow_to(frames);
            }
            Layer::of(
                track.name.clone(),
                &part.interleaved(),
                stereo::CHANNELS,
                SAMPLE_RATE,
            )
        })
        .collect()
}

/// Grows `buf` by however long `chain` needs to decay into.
///
/// A dry chain, and no chain at all, ask for nothing — so this is a no-op on
/// the path every existing song takes.
fn ring_out(buf: &mut Stereo, chain: &[Fx]) {
    let tail = (fx::tail_seconds(chain) * RATE).round() as usize;
    buf.resize(buf.frames() + tail);
}

/// Scales every sample of `buf` by a per-side gain.
fn scale(buf: &mut Stereo, (left, right): (f32, f32)) {
    for (slot, gain) in [(&mut buf.l, left), (&mut buf.r, right)] {
        for sample in slot.iter_mut() {
            *sample *= gain;
        }
    }
}

/// Adds `src` into `dst` starting at sample-frame `at`, at a per-side gain,
/// growing `dst` to fit.
///
/// The growth is what lets a song ring out: the last note's release tail
/// extends the buffer past the final beat instead of being truncated to it.
fn mix_into(dst: &mut Stereo, src: &Stereo, at: usize, (left, right): (f32, f32)) {
    let end = at + src.frames();
    dst.grow_to(end);
    for (slot, (source, gain)) in [(&mut dst.l, (&src.l, left)), (&mut dst.r, (&src.r, right))] {
        for (into, sample) in slot[at..end].iter_mut().zip(source) {
            *into += sample * gain;
        }
    }
}
