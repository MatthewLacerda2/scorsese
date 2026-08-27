//! core — the note renderer: patch + note → `f32` buffer.
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
//! (`vel_cutoff`) and at the FM modulator's depth (`vel_index`), both defaulting
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
//! Module layout: [`osc`] (band-limited oscillator stack), [`karplus`] (plucked
//! string), [`fm`] (2-op FM), [`noise`] (the one seeded RNG), [`source`] (which of
//! those runs), [`mod@env`] (ADSR), [`filter`] (state-variable filter).

pub(crate) mod env;
pub(crate) mod filter;
pub(crate) mod fm;
pub(crate) mod karplus;
pub(crate) mod noise;
pub(crate) mod osc;
pub(crate) mod source;

use std::f32::consts::TAU;

use super::error::SynthError;
use super::fx;
use super::note::{NoteOpts, midi_to_freq};
use super::patch::{Filter, Lfo, LfoTarget, Patch, PitchEnv};

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

/// Renders one note of `patch` at MIDI pitch `midi` under `opts`, returning the
/// raw `f32` buffer — **pre-limiter**, so a caller summing several notes can
/// limit the sum instead of each part of it.
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
pub fn render_note(patch: &Patch, midi: f32, opts: &NoteOpts) -> Result<Vec<f32>, SynthError> {
    patch.validate()?;
    let gate = gate_length(opts.duration)?;
    let n = sample_count(gate + patch.amp.r.max(0.0) + fx::tail_seconds(&patch.fx));
    let velocity = opts.velocity.clamp(0.0, 1.0);
    let brightness = (opts.velocity + opts.timbre).clamp(0.0, 1.0);

    let freqs = pitch_track(patch.lfo, patch.pitch_env, midi_to_freq(midi), gate, n);
    let mut buf = vec![0.0; n];
    source::render(&patch.source, &freqs, opts.seed, brightness, &mut buf, RATE);
    if let Some(f) = patch.filter {
        let cutoffs = cutoff_track(&f, patch.lfo, gate, n, brightness);
        filter::apply(&mut buf, &f, &cutoffs, RATE);
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
) -> Result<Vec<f32>, SynthError> {
    let mut buf = render_note(patch, midi, opts)?;
    fx::limiter::apply(&mut buf, RATE);
    Ok(buf)
}

/// Samples needed for `seconds` of audio, at least one and at most [`MAX_SECONDS`].
fn sample_count(seconds: f32) -> usize {
    ((seconds.clamp(0.0, MAX_SECONDS) * RATE).ceil() as usize).max(1)
}

/// The LFO's raw `−1..1` sine value at sample `i`.
fn lfo_wave(lfo: &Lfo, i: usize) -> f32 {
    (TAU * lfo.rate.max(0.0) * i as f32 / RATE).sin()
}

/// The largest pitch offset the track will carry, either way: ten octaves.
///
/// A guard, not a musical limit, and the same kind as [`MAX_SECONDS`]. The
/// offset is an exponent, so an unbounded one reaches `inf` Hz, and an `inf`
/// frequency becomes a `NaN` sample the moment [`fm`] takes the fractional part
/// of a phase. Ten octaves either way is past the audible band from any note,
/// so nothing that was making a sound is affected by the bound existing.
const MAX_PITCH_OFFSET: f32 = 120.0;

/// The per-sample frequency track: the played pitch, swept once by a pitch
/// envelope (`semitones` at full level) and bent cyclically by an LFO aimed at
/// `pitch` (`depth` semitones either way).
///
/// The two are summed **in semitones and then applied once**, rather than each
/// scaling the frequency in turn. That is the same "sources of movement add"
/// rule the cutoff already follows, and here it is also the only version that
/// is musically coherent: pitch is logarithmic, so semitones are what add, and
/// a sweep of −12 with a vibrato of ±1 is an octave drop with a semitone of
/// wobble on it however deep either one is.
///
/// A patch with neither takes the flat track, which is both faster and the
/// literal guarantee that a document written before this stage existed still
/// renders the frequency it always did.
fn pitch_track(
    lfo: Option<Lfo>,
    pitch_env: Option<PitchEnv>,
    base: f32,
    gate: f32,
    n: usize,
) -> Vec<f32> {
    let vibrato = lfo.filter(|l| l.target == LfoTarget::Pitch);
    let sweep = pitch_env.map(|p| (p.semitones, env::track(&p.adsr, gate, n, RATE)));
    if vibrato.is_none() && sweep.is_none() {
        return vec![base; n];
    }
    (0..n)
        .map(|i| {
            let mut semitones = 0.0;
            if let Some(l) = &vibrato {
                semitones += l.depth * lfo_wave(l, i);
            }
            if let Some((amount, envelope)) = &sweep {
                semitones += amount * envelope[i];
            }
            base * (semitones.clamp(-MAX_PITCH_OFFSET, MAX_PITCH_OFFSET) / 12.0).exp2()
        })
        .collect()
}

/// The per-sample cutoff track: the base cutoff, offset by how hard the note
/// was struck (`vel_cutoff` Hz at full velocity), swept by the filter envelope
/// (`env_amount` Hz at full level) and wobbled by an LFO aimed at `cutoff`
/// (`depth` octaves either way).
///
/// The velocity offset joins the sum rather than scaling it, and lands *inside*
/// the LFO's multiply: velocity says how bright this strike is, and the wobble
/// then rides that brightness in octaves either way, as it already did the
/// envelope's. Nothing here bounds the result — [`filter::apply`] clamps every
/// cutoff into the SVF's stable band anyway, and doing it twice would only
/// invite the two clamps to disagree.
fn cutoff_track(f: &Filter, lfo: Option<Lfo>, gate: f32, n: usize, velocity: f32) -> Vec<f32> {
    let envelope = env::track(&f.adsr, gate, n, RATE);
    let struck = f.cutoff + f.vel_cutoff * velocity;
    (0..n)
        .map(|i| {
            let swept = struck + f.env_amount * envelope[i];
            match lfo {
                Some(l) if l.target == LfoTarget::Cutoff => {
                    swept * (l.depth * lfo_wave(&l, i)).exp2()
                }
                _ => swept,
            }
        })
        .collect()
}

/// Apply velocity, the amp envelope and any tremolo — the stage that turns a
/// continuous tone into a note. `velocity` arrives already clamped from
/// [`render_note`].
fn apply_amp(buf: &mut [f32], patch: &Patch, gate: f32, velocity: f32) {
    let envelope = env::track(&patch.amp, gate, buf.len(), RATE);
    for (i, s) in buf.iter_mut().enumerate() {
        *s *= velocity * envelope[i] * tremolo(patch.lfo, i);
    }
}

/// The tremolo gain at sample `i`: an LFO aimed at `amp` dips the level by `depth`
/// (so `depth = 1` dips all the way to silence).
fn tremolo(lfo: Option<Lfo>, i: usize) -> f32 {
    match lfo {
        Some(l) if l.target == LfoTarget::Amp => {
            1.0 - l.depth.clamp(0.0, 1.0) * 0.5 * (1.0 - lfo_wave(&l, i))
        }
        _ => 1.0,
    }
}

/// The pitch-bend path by the number, not the range.
///
/// What defends these two functions otherwise is a suite that asks whether a
/// note has audio in it and whether the level moves — which has no opinion
/// about *which* note is playing. A wrong operator here ships sound that is
/// wrong rather than sound that is absent, in the crate whose whole promise is
/// that one recipe always makes one file.
///
/// Every expected value below is worked out by hand from what the two
/// functions document and written as a literal; recomputing the formula in the
/// test would only assert that the code agrees with itself.
///
/// **On exactness.** These are `f32`, and each assertion says which it is:
/// `assert_eq!` where the maths is exact in binary — `sin(0)`, `2^0 = 1`,
/// `2^±1 = 2` and `0.5`, and the identity multiplies through them — and a tight
/// epsilon only where an irrational lands between two floats, with the reason
/// named. A tolerance wide enough to hide a swapped operator would defeat the
/// point of the file.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::Adsr;

    /// The pitch every bend below is measured from: A4, and a round number of
    /// Hz, so an octave up and an octave down are round numbers too.
    const BASE: f32 = 440.0;

    /// The rate every wave below runs at. **Not 1 Hz**: `TAU * rate` and
    /// `TAU / rate` are the same expression at 1, so a rate of one would let
    /// that mutation through. 5 Hz also divides [`RATE`] evenly four ways,
    /// which is what makes the quarter-cycle index an integer.
    const RATE_HZ: f32 = 5.0;

    /// A quarter of a cycle at [`RATE_HZ`]: `44_100 / (4 * 5)`. The peak.
    const PEAK: usize = 2_205;

    /// Three quarters of a cycle: the trough.
    const TROUGH: usize = 6_615;

    /// A whole cycle, and one sample longer than the tracks need to be.
    const CYCLE: usize = 8_820;

    fn lfo(depth: f32, target: LfoTarget) -> Lfo {
        Lfo {
            rate: RATE_HZ,
            depth,
            target,
        }
    }

    /// The frequency track a vibrato alone produces. The gate is a second —
    /// long enough to be irrelevant, since with no sweep nothing reads it.
    fn vibrato_track(lfo: Option<Lfo>, n: usize) -> Vec<f32> {
        pitch_track(lfo, None, BASE, 1.0, n)
    }

    /// A pitch envelope of `semitones` under `adsr`.
    fn sweep(semitones: f32, adsr: Adsr) -> PitchEnv {
        PitchEnv { semitones, adsr }
    }

    /// The drum shape: full offset immediately, decaying to the played note
    /// over one second. Straight, so the level at a moment is a number this
    /// file can state rather than derive.
    fn falling() -> Adsr {
        Adsr {
            a: 0.0,
            d: 1.0,
            s: 0.0,
            r: 0.0,
            curve: 0.0,
        }
    }

    /// An envelope pinned at full level for the whole gate. Not a shape a drum
    /// wants, but the one that makes every sum below a round number.
    fn held() -> Adsr {
        Adsr {
            a: 0.0,
            d: 0.0,
            s: 1.0,
            r: 0.0,
            curve: 0.0,
        }
    }

    /// The frequency track a sweep alone produces, over a one-second gate so a
    /// sample index divided by [`RATE`] *is* the envelope's progress.
    fn sweep_track(semitones: f32, adsr: Adsr, n: usize) -> Vec<f32> {
        pitch_track(None, Some(sweep(semitones, adsr)), BASE, 1.0, n)
    }

    /// A downward sweep starts high and arrives on the played note. Twenty-four
    /// semitones is two octaves, so half a straight decay is one octave and A4
    /// is A5 — both exact, because `2^2` and `2^1` are.
    ///
    /// The very first sample is the played pitch rather than the top of the
    /// sweep: [`env::level_at`] is zero at `t = 0` for every envelope there
    /// is, which is what keeps an amp envelope from starting with a step. One
    /// sample is 23 µs, so the sweep is audibly at its top from the start.
    #[test]
    fn a_pitch_envelope_starts_off_the_note_and_lands_on_it() {
        let track = sweep_track(24.0, falling(), 44_100);
        assert_eq!(track[0], BASE, "the envelope is still at zero");
        assert!(track[1] > 1_759.0, "two octaves up, at {}", track[1]);
        assert_eq!(track[22_050], 880.0, "one octave left at half the decay");
        assert!(
            (track[44_099] - BASE).abs() < 0.1,
            "and settles on the note, at {}",
            track[44_099]
        );
    }

    /// Negative sweeps the other way — up onto the note from below, not the
    /// same direction scaled.
    #[test]
    fn a_negative_amount_sweeps_up_onto_the_note_from_below() {
        let track = sweep_track(-12.0, held(), 8);
        assert_eq!(track[1], 220.0, "an octave below at full level");
    }

    /// The two modulators add **in semitones**: at the vibrato's peak the two
    /// twelves make two octaves, and at its trough they cancel exactly. A
    /// version that multiplied frequencies, or that let one replace the other,
    /// moves both numbers.
    #[test]
    fn a_sweep_and_a_vibrato_add_in_semitones() {
        let track = pitch_track(
            Some(lfo(12.0, LfoTarget::Pitch)),
            Some(sweep(12.0, held())),
            BASE,
            1.0,
            CYCLE,
        );
        assert_eq!(track[PEAK], 1_760.0, "an octave of sweep plus one of bend");
        assert_eq!(track[TROUGH], BASE, "and the bend cancels the sweep");
    }

    /// A pitch envelope with no LFO at all still sweeps — the vibrato is not
    /// the thing that switches the modulated path on.
    #[test]
    fn a_sweep_needs_no_lfo_to_reach_the_track() {
        assert_eq!(sweep_track(12.0, held(), 8)[1], 880.0);
    }

    /// Ten octaves is the ceiling, so an amount past it cannot reach `inf` Hz —
    /// and an `inf` frequency is a `NaN` sample as soon as a phase is wrapped.
    #[test]
    fn an_absurd_sweep_is_clamped_instead_of_reaching_infinity() {
        for semitones in [1e9, -1e9, f32::MAX, f32::MIN] {
            let track = sweep_track(semitones, held(), 16);
            for (i, f) in track.iter().enumerate() {
                assert!(f.is_finite() && *f > 0.0, "{semitones} at {i} gave {f}");
            }
        }
    }

    /// `sin(0)` is zero however fast the LFO is running, so the first sample
    /// pins the rate term out of the way of everything after it.
    #[test]
    fn the_wave_starts_at_zero_whatever_the_rate() {
        for rate in [0.0, 0.1, 1.0, RATE_HZ, 440.0] {
            let l = Lfo {
                rate,
                depth: 1.0,
                target: LfoTarget::Pitch,
            };
            assert_eq!(lfo_wave(&l, 0), 0.0, "rate {rate}");
        }
    }

    /// A quarter cycle in, the phase is `TAU/4` and the sine is at its peak;
    /// three quarters in, its trough. Both are exact: the nearest `f32` to
    /// `sin` of the nearest `f32` to a right angle is 1 itself, because the
    /// sine is flat there.
    #[test]
    fn a_quarter_cycle_in_the_wave_is_at_its_peak() {
        let l = lfo(1.0, LfoTarget::Pitch);
        assert_eq!(lfo_wave(&l, PEAK), 1.0);
        assert_eq!(lfo_wave(&l, TROUGH), -1.0);
    }

    /// A whole cycle is back at zero — the one value here that cannot be
    /// exact, because `TAU` is irrational and the `f32` nearest it is a
    /// fraction of a degree past the turn. The error is the gap in `TAU`, not
    /// slack for an operator to hide in.
    #[test]
    fn a_whole_cycle_returns_to_zero() {
        assert!(lfo_wave(&lfo(1.0, LfoTarget::Pitch), CYCLE).abs() < 1e-6);
    }

    /// A negative rate is clamped to nothing rather than run backwards, so the
    /// wave is held at zero for the whole track.
    #[test]
    fn a_negative_rate_holds_the_wave_still() {
        let l = Lfo {
            rate: -RATE_HZ,
            depth: 1.0,
            target: LfoTarget::Pitch,
        };
        for i in [0, PEAK, TROUGH, CYCLE] {
            assert_eq!(lfo_wave(&l, i), 0.0, "sample {i}");
        }
    }

    /// The wave is zero at the first sample, so the bend is `2^0 = 1` and the
    /// pitch is the one that was played — for any depth at all, including one
    /// deep enough to be audible two octaves away.
    #[test]
    fn the_first_sample_is_the_played_pitch_whatever_the_depth() {
        for depth in [0.0, 2.0, 12.0, -7.0, 24.0] {
            let track = vibrato_track(Some(lfo(depth, LfoTarget::Pitch)), 8);
            assert_eq!(track[0], BASE, "depth {depth}");
        }
    }

    /// Twelve semitones is an octave: at the peak the bend is `2^(12/12) = 2`
    /// and A4 is A5, at the trough it is A3. Both numbers move under every
    /// mutation of the semitones-to-octaves divide, of the depth multiply and
    /// of the multiply that applies the bend to the pitch.
    #[test]
    fn twelve_semitones_bends_a_whole_octave_each_way() {
        let track = vibrato_track(Some(lfo(12.0, LfoTarget::Pitch)), CYCLE);
        assert_eq!(track[PEAK], 880.0);
        assert_eq!(track[TROUGH], 220.0);
    }

    /// Twice the depth is twice the octaves, not twice the frequency — the
    /// difference between a bend that is exponential in semitones and one that
    /// is anything else.
    #[test]
    fn twice_the_depth_is_twice_the_octaves() {
        let track = vibrato_track(Some(lfo(24.0, LfoTarget::Pitch)), CYCLE);
        assert_eq!(track[PEAK], 1760.0);
        assert_eq!(track[TROUGH], 110.0);
    }

    /// An LFO aimed at the filter or the amplifier does not touch the pitch,
    /// however deep it is — the whole track is the played note.
    #[test]
    fn an_lfo_aimed_elsewhere_leaves_the_pitch_flat() {
        for target in [LfoTarget::Cutoff, LfoTarget::Amp] {
            flat(&vibrato_track(Some(lfo(12.0, target)), CYCLE), CYCLE);
        }
    }

    /// And no LFO at all is the same flat track, at the length asked for.
    #[test]
    fn no_lfo_is_a_flat_track_of_the_length_asked_for() {
        flat(&vibrato_track(None, 512), 512);
    }

    /// `n` samples, every one of them the played pitch. Sample by sample
    /// rather than against a whole `Vec`, so a track that bends names the
    /// sample that first did instead of printing several thousand floats.
    fn flat(track: &[f32], n: usize) {
        assert_eq!(track.len(), n);
        for (i, f) in track.iter().enumerate() {
            assert_eq!(*f, BASE, "sample {i}");
        }
    }
}
