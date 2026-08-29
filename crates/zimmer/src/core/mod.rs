//! core — the note renderer: patch + note → a stereo pair of `f32` buffers.
//!
//! This is the layer's contract made executable: *(pitch, velocity, duration) in →
//! buffer out*. The signal path is fixed, which is the whole point of a structured
//! patch — the recipe picks what fills each stage, never how they connect:
//!
//! ```text
//!   source ─► filter ─► amp envelope ─► fx chain
//!      ▲         ▲            ▲
//!      ├──── envelopes ───────┘   (amp always; pitch and cutoff optional)
//!      └─────── LFO ──────────┘   (one target: pitch | cutoff | amp)
//! ```
//!
//! An envelope has three possible destinations — the amplifier always, and
//! optionally the filter cutoff and the note's own pitch. The pitch envelope is
//! the one-shot the LFO is not: a sweep that happens once and settles, which is
//! what a kick drum, a tom, an 808 and a laser zap all are. It writes into the
//! same per-sample frequency track a vibrato does, in semitones, and the two
//! **add** — so a note may fall onto its pitch and then wobble around it.
//!
//! A note's **velocity** taps that path in up to three places, not one. It
//! always scales the amp envelope; a patch may also aim it at the filter cutoff
//! (`vel_octaves`) and at the FM modulator's depth (`vel_index`), both defaulting
//! to zero. That is what separates a note that was *played* from one that was
//! turned up — striking an instrument harder moves energy into the upper
//! harmonics, and a velocity that only reaches a fader is the reason a
//! carefully written part can still sound mechanical.
//!
//! Those last two are shown a velocity of their **own**: the same number,
//! unless a performance moved this strike's tone off its level, which is what
//! [`NoteOpts::timbre`] carries. Tone and loudness are two hands, and a part
//! whose every note brightens exactly as much as it gets louder is a part
//! played with one.
//!
//! Everything is `f32` in `−1..=1` end to end; the only quantisation is the WAV
//! encoder's. The rendered buffer is **longer than the note**: the amp
//! envelope's release rings out after the gate closes, and an fx chain adds its
//! own tail (an echo cut off mid-repeat is a click, not an echo).
//!
//! **Both channels take the whole path**, from the source stage onward. For
//! every source but `noise` the two are computed from identical inputs and so
//! come out bit-identical, which is arithmetic done twice for one answer — and
//! that is the price of the path being uniform. The alternative was a flag
//! saying whether the sides had parted company yet, read by every stage
//! downstream, which is a mode; this is an offline renderer and it can afford
//! the multiply instead. What it buys is that the fx chain, where a stereo
//! reverb lives, needs no special case for the one-shot that reached it dry.
//!
//! Module layout: [`osc`] (band-limited oscillator stack), [`karplus`] (plucked
//! string), [`fm`] (FM, two operators or four), [`additive`] (a stated
//! harmonic series), [`noise`] (the one seeded RNG), [`nyquist`] (what any of
//! them may place above the played pitch), [`source`] (which of those runs),
//! [`mod@env`] (ADSR), [`filter`] (state-variable filter), [`tracks`] (the
//! per-sample curves the stages walk: pitch, cutoff, tremolo).

pub(crate) mod additive;
pub(crate) mod env;
pub(crate) mod filter;
pub(crate) mod fm;
pub(crate) mod karplus;
pub(crate) mod noise;
pub(crate) mod nyquist;
pub(crate) mod osc;
pub(crate) mod source;
pub(crate) mod tracks;

use super::error::SynthError;
use super::fx;
use super::note::{NoteOpts, midi_to_freq};
use super::patch::Patch;
use super::stereo::Stereo;
use tracks::{cutoff_track, pitch_track, tremolo};

/// The one rate everything here renders at. 44.1 kHz is CD rate: the full
/// audible band.
///
/// Fixed rather than chosen per call, and deliberately so. A bake is addressed
/// by the hash of the recipe that made it, so it must not vary with what some
/// later render happens to ask for — and `scorsese-render` resamples every
/// audio source on the way into the mix anyway. The reverb's delay lines are
/// tuned against this number; changing it would detune them silently.
pub const SAMPLE_RATE: u32 = 44_100;

/// [`SAMPLE_RATE`] as the float the DSP works in.
pub(crate) const RATE: f32 = SAMPLE_RATE as f32;

/// Longest buffer one note may render to. A guard, not a musical limit: a typo in
/// `duration` should fail loudly rather than allocate for an hour of audio.
const MAX_SECONDS: f32 = 60.0;

/// Renders one note of `patch` at MIDI pitch `midi` under `opts`, returning
/// the raw stereo pair — **pre-limiter**, so a caller summing several notes
/// can limit the sum instead of each part of it.
///
/// Velocity is clamped once, here, and then handed to every stage that reads
/// it. Three stages now do — the FM source, the filter and the amp envelope —
/// and if each clamped for itself, one of them eventually would not, and a
/// note would be struck at three subtly different strengths at once.
///
/// It arrives as **two** numbers rather than one, and they part company only
/// by [`NoteOpts::timbre`]: the level the amp envelope multiplies by, and the
/// velocity the brightness routings are shown. A player varies tone and
/// loudness separately — bow pressure against bow speed — and the routings are
/// where that difference already lives. They are clamped in the same place, to
/// the same band, for the reason above; `timbre` of zero collapses them back
/// into the single number every note was struck at before.
pub(crate) fn render_note(patch: &Patch, midi: f32, opts: &NoteOpts) -> Result<Stereo, SynthError> {
    patch.validate()?;
    let gate = gate_length(opts.duration)?;
    let n = sample_count(gate + patch.amp.r.max(0.0) + fx::tail_seconds(&patch.fx));
    let velocity = opts.velocity.clamp(0.0, 1.0);
    let brightness = (opts.velocity + opts.timbre).clamp(0.0, 1.0);

    let freqs = pitch_track(
        patch.lfo,
        patch.pitch_env,
        opts.glide,
        midi_to_freq(midi),
        gate,
        n,
    );
    let mut buf = source::render(&patch.source, &freqs, opts.seed, brightness, gate, n, RATE);
    if let Some(f) = patch.filter {
        // One cutoff track, both channels: the filter's *state* belongs to a
        // channel, and the curve it is following belongs to the note.
        let cutoffs = cutoff_track(&f, patch.lfo, gate, n, brightness);
        buf.each(|channel| filter::apply(channel, &f, &cutoffs, RATE));
    }
    apply_amp(&mut buf, patch, gate, velocity);
    fx::apply_chain(&mut buf, &patch.fx, RATE);
    Ok(buf)
}

/// Validates the requested note length, rejecting the values that would render
/// nothing at all.
fn gate_length(duration: f32) -> Result<f32, SynthError> {
    if !duration.is_finite() || duration <= 0.0 {
        return Err(SynthError::BadDuration { duration });
    }
    Ok(duration)
}

/// Renders one note and limits it — what a single baked one-shot needs, so it
/// cannot clip its own file.
///
/// Kept apart from [`render_note`] because the song mixer wants the unlimited
/// form: limiting every note before summing them would squash each one's
/// dynamics and then squash the sum again.
pub(crate) fn render_limited(
    patch: &Patch,
    midi: f32,
    opts: &NoteOpts,
) -> Result<Stereo, SynthError> {
    let mut buf = render_note(patch, midi, opts)?;
    fx::limiter::apply(&mut buf, RATE);
    Ok(buf)
}

/// Samples needed for `seconds` of audio, at least one and at most [`MAX_SECONDS`].
fn sample_count(seconds: f32) -> usize {
    ((seconds.clamp(0.0, MAX_SECONDS) * RATE).ceil() as usize).max(1)
}

/// Apply velocity, the amp envelope and any tremolo — the stage that turns a
/// continuous tone into a note. `velocity` arrives already clamped from
/// [`render_note`].
fn apply_amp(buf: &mut Stereo, patch: &Patch, gate: f32, velocity: f32) {
    let envelope = env::track(&patch.amp, gate, buf.frames(), RATE);
    // The gain curve is worked out once and both channels are walked with it:
    // an envelope is a property of the note, and a tremolo that ran at a
    // slightly different phase on each side would be a stereo effect nobody
    // asked for.
    let gains: Vec<f32> = (0..buf.frames())
        .map(|i| velocity * envelope[i] * tremolo(patch.lfo, i))
        .collect();
    buf.each(|channel| {
        for (s, gain) in channel.iter_mut().zip(&gains) {
            *s *= gain;
        }
    });
}
