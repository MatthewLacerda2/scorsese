//! The stages a patch is built from: what makes the tone, what shapes it, and
//! what is done to it afterwards.
//!
//! Every type here is plain serde data — no buffers, no handles — so a patch
//! round-trips losslessly through JSON and means the same thing whether it was
//! written by hand or by an agent.

use serde::{Deserialize, Serialize};

/// The most oscillators one stack may carry. Four detuned saws is already a
/// fat analog lead; past that it is CPU for no audible gain.
pub const MAX_OSCS: usize = 4;

/// One oscillator's waveform.
///
/// `saw` and `square` are band-limited (polyBLEP) at render time — the naive
/// versions alias audibly at high pitch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wave {
    /// A pure tone: one harmonic, nothing for a filter to work on.
    Sine,
    /// Odd harmonics falling away steeply. Soft, flute-ish.
    Triangle,
    /// Every harmonic. The classic subtractive starting point.
    Saw,
    /// Odd harmonics only — hollow, and the head of most chiptune leads.
    Square,
}

/// One oscillator in a stack: its wave, its detune from the played pitch, its
/// weight in the mix, and a whole-octave transpose.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Osc {
    /// Which waveform this oscillator produces.
    pub wave: Wave,
    /// Detune in cents (100 cents = a semitone). Small opposing detunes across
    /// a stack are what make a "fat" lead.
    #[serde(default)]
    pub detune_cents: f32,
    /// Weight in the stack mix; the stack is normalised by the sum of gains.
    #[serde(default = "one")]
    pub gain: f32,
    /// Whole-octave transpose, e.g. `-1` for a sub-oscillator.
    #[serde(default)]
    pub octave: i32,
}

/// What generates the raw tone. Exactly one per patch — the head of the fixed
/// signal path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    /// Subtractive stack: up to [`MAX_OSCS`] oscillators summed, the classic
    /// analog-synth head.
    OscStack {
        /// The oscillators, summed and normalised by the sum of their gains.
        oscs: Vec<Osc>,
    },
    /// Karplus-Strong plucked string: a noise-excited delay line with damped
    /// feedback. Guitars, basses, harps, marimbas.
    Karplus {
        /// Feedback scale per round trip, `0..1` — how long the string rings.
        #[serde(default = "damping_default")]
        damping: f32,
        /// Excitation brightness, `0..1`: 0 is a soft, pre-lowpassed pluck, 1
        /// is full white noise (a hard pick).
        #[serde(default = "half")]
        brightness: f32,
    },
    /// Raw white noise — the head of every gunshot, impact and footstep,
    /// shaped by the filter and amp envelope.
    Noise,
    /// Two-operator FM: a sine modulator at `ratio × f` bending a sine
    /// carrier. Electric pianos, bells, metallic hits.
    Fm2 {
        /// Modulator frequency as a multiple of the played pitch. Integer
        /// ratios are harmonic (tonal); non-integer ratios go inharmonic
        /// (metallic).
        ratio: f32,
        /// Modulation depth. Higher indices add sidebands, i.e. brightness.
        index: f32,
        /// How much modulation depth a full-velocity strike adds to `index`.
        /// Defaults to `0.0`: brightness fixed however hard the note is
        /// struck, which is what every patch written before this field
        /// existed already meant.
        ///
        /// The FM half of the velocity routing described on
        /// [`Filter::vel_cutoff`], and the more literal half: in two-operator
        /// FM the index *is* the brightness, so this is the whole difference
        /// between a bell tapped and a bell hit.
        ///
        /// Added to `index` and then floored at zero. The floor is not
        /// defensive — a negative index only mirrors the modulator, and a
        /// mirrored modulator sounds exactly as bright as its positive twin,
        /// so an unclamped sum would make a darkening routing start
        /// brightening again the moment it crossed over.
        #[serde(default)]
        vel_index: f32,
        /// Seconds for the modulator's own decay — how fast the bright attack
        /// collapses to a near-sine body.
        #[serde(default = "mod_decay_default")]
        mod_decay: f32,
    },
}

impl Source {
    /// What this generator is called, in the word the document spells it with.
    ///
    /// The serde tag rather than a prose label, so a report that names a source
    /// and a recipe that chooses one use the same vocabulary — a reader who
    /// sees `karplus` in a report can search the recipes for it.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::OscStack { .. } => "osc_stack",
            Self::Karplus { .. } => "karplus",
            Self::Noise => "noise",
            Self::Fm2 { .. } => "fm2",
        }
    }
}

/// A linear-segment ADSR envelope.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Adsr {
    /// Attack: seconds from silence to full level.
    pub a: f32,
    /// Decay: seconds from full level down to `s`.
    pub d: f32,
    /// Sustain: the level held while the note is, in `0..=1`.
    pub s: f32,
    /// Release: seconds from the sustain level back to silence once the gate
    /// closes. This is why a rendered note is longer than its duration.
    pub r: f32,
}

impl Default for Adsr {
    /// A quick, fully-sustaining envelope — an organ-ish "just pass it
    /// through".
    fn default() -> Self {
        Self {
            a: 0.005,
            d: 0.0,
            s: 1.0,
            r: 0.05,
        }
    }
}

/// Which side of the filter is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterKind {
    /// Keep what is below the cutoff: the darkening move.
    Lowpass,
    /// Keep what is above it: thins a sound out, and is how a small speaker
    /// gets simulated.
    Highpass,
}

/// The optional filter stage: a Chamberlin state-variable filter whose cutoff
/// is swept by its own envelope — the move that makes a subtractive patch
/// expressive rather than static.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    /// Which side of the cutoff survives.
    pub kind: FilterKind,
    /// Base cutoff in Hz.
    pub cutoff: f32,
    /// Resonance in `0..1`: emphasis at the cutoff. Near 1 the filter
    /// self-rings.
    #[serde(default)]
    pub resonance: f32,
    /// How many Hz the filter envelope adds to the cutoff at full level.
    /// Negative sweeps downward.
    #[serde(default)]
    pub env_amount: f32,
    /// How many Hz a full-velocity strike adds to the cutoff. Defaults to
    /// `0.0`, which is velocity doing nothing here — exactly what every patch
    /// written before this field existed already meant.
    ///
    /// This is what makes a note read as *played* rather than *turned up*. On
    /// any real instrument, more energy in means more energy in the upper
    /// harmonics: a hard-picked string is not merely a louder string, it is a
    /// brighter one, and the ear reads that change in brightness as effort.
    /// Velocity aimed only at the fader is a large part of why a carefully
    /// written synthesised part still sounds like a machine.
    ///
    /// Same Hz unit and same sign convention as [`Filter::env_amount`],
    /// because it is the same quantity arriving from a different source — one
    /// mental model, and the two are directly comparable when both are set.
    /// The terms are **added**, `cutoff + env_amount × env + vel_cutoff × vel`,
    /// so each stays independent of the others and a zero stays harmless;
    /// multiplying would let `vel = 0` shut the filter outright, which is a
    /// different and worse instrument.
    ///
    /// Negative is legal and means velocity *darkens* — a perfectly good
    /// instrument, and the reason this is not validated as positive. The
    /// resulting cutoff is clamped into the filter's stable band per sample,
    /// exactly as a negative `env_amount`'s already is, so no value here can
    /// produce an unstable filter.
    #[serde(default)]
    pub vel_cutoff: f32,
    /// The cutoff envelope. Defaults to the same fast shape as
    /// [`Adsr::default`].
    #[serde(default)]
    pub adsr: Adsr,
}

/// What the LFO modulates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LfoTarget {
    /// Vibrato: `depth` semitones of pitch bend either way.
    Pitch,
    /// Filter wobble: `depth` octaves of cutoff sweep either way.
    Cutoff,
    /// Tremolo: amplitude dips by `depth` (1.0 dips to silence).
    Amp,
}

/// A single sine low-frequency oscillator tapping one target.
///
/// One is enough for vibrato, wobble or tremolo. A modulation matrix would be
/// a free graph, which is exactly the shape a patch deliberately is not.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lfo {
    /// Rate in Hz, typically 0.1–10.
    pub rate: f32,
    /// Modulation depth; its unit depends on [`LfoTarget`].
    pub depth: f32,
    /// Which stage of the signal path it bends.
    pub target: LfoTarget,
}

/// One effect in the post-chain, applied in list order.
///
/// A limiter is *always* applied at bake and is deliberately not listed here —
/// it is not a choice the recipe gets to make.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "fx", rename_all = "snake_case")]
pub enum Fx {
    /// Feedback echo.
    Delay {
        /// Echo spacing in seconds.
        time: f32,
        /// How much of each echo feeds the next, `0..1`.
        feedback: f32,
        /// Wet/dry blend, `0..=1`.
        mix: f32,
    },
    /// Freeverb room reverb: eight combs and four allpasses.
    Reverb {
        /// Room size, `0..=1`.
        size: f32,
        /// High-frequency damping, `0..=1` — how quickly the tail dulls.
        damp: f32,
        /// Wet/dry blend, `0..=1`.
        mix: f32,
    },
    /// A `tanh` soft clip: the one nonlinearity in the signal path, and so the
    /// only stage that can put a harmonic into the output that was not in the
    /// input. What "warm" and "glued" are made of.
    Saturate {
        /// How hard the signal is pushed into the curve. `0` is the identity
        /// line, `1`–`2` is warmth, `4` is audible drive; clamped at a ceiling
        /// past which a soft clip is a fuzz pedal. It is gain-compensated, so
        /// this changes the shape of the wave and not its peak.
        drive: f32,
        /// Wet/dry blend, `0..=1`. Parallel drive — a hard-driven copy under a
        /// clean one — is how weight is added without rounding the transients
        /// off, and it costs nothing over a full-wet setting.
        mix: f32,
    },
}

fn one() -> f32 {
    1.0
}

fn half() -> f32 {
    0.5
}

fn damping_default() -> f32 {
    0.996
}

fn mod_decay_default() -> f32 {
    0.3
}
