//! The stages a patch is built from: what makes the tone, what shapes it, and
//! what is done to it afterwards.
//!
//! Every type here is plain serde data — no buffers, no handles — so a patch
//! round-trips losslessly through JSON and means the same thing whether it was
//! written by hand or by an agent.

use serde::{Deserialize, Serialize};

use super::fm::{Algorithm, FM_OPERATORS, Operator};
use crate::level::bands;

/// The most oscillators one stack may carry. Four detuned saws is already a
/// fat analog lead; past that it is CPU for no audible gain.
pub const MAX_OSCS: usize = 4;

/// The most detuned copies one oscillator may sound at once.
///
/// Seven, which is the supersaw's own number and not a round one: the voice
/// count the JP-8000 shipped with is what the sound is, and every unison patch
/// written since has been reaching for it. Past seven the copies land closer
/// together than the ear separates them, and the arithmetic is the whole
/// oscillator again — the same argument [`MAX_OSCS`] makes, applied one level
/// down, where a stack of four at seven voices is already twenty-eight
/// oscillators for one note.
pub const MAX_VOICES: usize = 7;

/// The most partials one additive series may carry.
///
/// Sixteen, and the number is argued the way [`MAX_OSCS`] is rather than
/// copied from it. Sixteen partials cover the whole audible harmonic series of
/// any note up to about 1.3 kHz; above that the top of the series is over
/// 22 kHz and is dropped at Nyquist regardless of this cap. Below it, the
/// seventeenth harmonic of any timbre this crate is for sits far enough down
/// that nothing is missed by its absence — a drawbar organ offers nine, and
/// the ear stops separating partials well before it stops hearing them.
///
/// Past the cap it is one more sine oscillator per note for no audible gain,
/// and an uncapped list is an unbounded allocation, which is one of the few
/// things this crate refuses outright.
pub const MAX_PARTIALS: usize = 16;

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

/// How a noise source's energy is spread across the spectrum.
///
/// Named for the slope rather than the use, because one colour is many
/// sounds: pink is wind, surf, rain and room tone as well as the body of a
/// snare. What each one *is* — the filter behind it, the measured slope, and
/// why a lowpass over white is not a substitute for either — is
/// `core::noise::color`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoiseColor {
    /// Equal energy per hertz, so most of it is in the top octave: a hiss.
    /// Cymbals, static, air.
    #[default]
    White,
    /// −3 dB per octave — equal energy per octave, the balance most natural
    /// sound falls in. Wind, surf, rain, room tone, and a far better snare or
    /// cymbal body than white under a filter.
    Pink,
    /// −6 dB per octave, an integrated draw. Thunder, rumble, distant
    /// traffic, and the low half of an impact.
    Brown,
}

/// One oscillator in a stack: its wave, its detune from the played pitch, its
/// weight in the mix, a whole-octave transpose, and how many detuned copies of
/// itself it sounds at once.
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
    /// How many detuned copies of this oscillator sound at once — unison.
    ///
    /// `1`, the default, is one oscillator and is exactly what every patch
    /// written before this field existed already meant. Past that the entry
    /// becomes `voices` copies spaced evenly across [`Osc::spread`], each with
    /// its own start phase, which is the supersaw.
    ///
    /// **Unison lives here rather than in the stack** because it is one timbre
    /// and not several. Written out as separate oscillators it costs the whole
    /// [`MAX_OSCS`] budget on a single sound and leaves nothing for the
    /// sub-oscillator or the second wave that a patch actually wants beside
    /// it, and every one of those entries repeats the same wave, gain and
    /// octave to say one thing.
    ///
    /// **It is a thickness control and not a fader.** The copies are
    /// normalised by their own count, so `voices: 7` is the same weight in the
    /// stack that `voices: 1` was; without that, adding unison to one
    /// oscillator would move every gain in the song.
    ///
    /// At most [`MAX_VOICES`], and at least one — a stack refuses both ends
    /// rather than quietly rendering something the recipe did not write.
    #[serde(default = "solo", skip_serializing_if = "is_solo")]
    pub voices: usize,
    /// How far apart the outermost unison voices sit, in cents — the full
    /// width of the spread, not the distance from the centre.
    ///
    /// Read only when [`Osc::voices`] is above one, and centred on
    /// [`Osc::detune_cents`], so a spread never moves the pitch the
    /// oscillator was written at. The default is a shimmer; `25` is a fat
    /// lead and past `50` the voices start to be heard as a chord rather than
    /// as one thick note.
    #[serde(default = "spread_default", skip_serializing_if = "is_default_spread")]
    pub spread: f32,
}

/// One partial of an additive series: where it sits, how much of it there is,
/// how far off the harmonic grid it is bent, and how fast it dies on its own.
///
/// A drawbar, in other words — and that is the whole idea of the source. Every
/// other head of the signal path generates something rich and lets a filter
/// carve it down; a table of these *states* the spectrum, so the timbre is the
/// numbers rather than what a filter shape happened to arrive at.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Partial {
    /// Frequency as a multiple of the played pitch: `1` is the fundamental,
    /// `2` the octave above it, `3` the twelfth above that.
    ///
    /// Whole numbers keep the series harmonic, which is what a tone with a
    /// definite pitch is made of. Fractional ones are legal and go inharmonic
    /// — a bell, a struck plate, a glass — and unlike the fractional ratio an
    /// [`Source::Fm2`] modulator takes, each partial is *placed* rather than
    /// arriving as a sideband of one number chosen for its overall effect.
    ///
    /// A ratio at or below zero is refused: it names no frequency, and zero in
    /// particular is a DC offset rather than a sound.
    pub ratio: f32,
    /// Weight in the mix. The series is normalised by the sum of the gains of
    /// the partials that actually sound, so adding one thickens the tone
    /// without making it louder.
    #[serde(default = "one")]
    pub gain: f32,
    /// Detune off `ratio`, in cents (100 cents = a semitone).
    ///
    /// The route to *chosen* inharmonicity. Real strings are slightly
    /// stretched — a piano's upper partials sit progressively sharp of the
    /// whole numbers, and that stretch is a good part of why a piano sounds
    /// like a piano while a perfectly harmonic stack sounds like an organ. A
    /// few cents more on each partial than the last is exactly that, written
    /// down.
    ///
    /// Cents rather than Hz for the reason [`PitchEnv::semitones`] gives:
    /// pitch is logarithmic, so a detune in Hz would mean a different interval
    /// at every pitch the patch is played at.
    #[serde(default)]
    pub detune_cents: f32,
    /// Seconds for this partial's **own** exponential decay, underneath the
    /// patch's amp envelope. `0` — the default — is a partial that does not
    /// fade by itself.
    ///
    /// This is the field the source is really for. On anything bowed, blown or
    /// struck the upper partials die sooner than the fundamental, and a tone
    /// whose partials all fade together is the one thing no filter sweep quite
    /// fakes. Give the fundamental a long decay and the sixth partial a short
    /// one and the note is bright at its attack and settles into its body,
    /// which is what a real one does.
    ///
    /// The two envelopes **multiply**, and the shorter always wins. The
    /// renderer's own module doc states that relationship in full; the short
    /// version is that a patch letting the partials do the shaping wants an
    /// amp envelope that sustains, or the note dies twice.
    #[serde(default)]
    pub decay: f32,
}

/// What generates the raw tone. Exactly one per patch — the head of the fixed
/// signal path.
///
/// Six of them, in three groups, and they are listed in that order because it
/// is also how a recipe chooses:
///
/// - **Make something rich and carve it.**
///   [`OscStack`](Source::OscStack) generates every harmonic,
///   [`Karplus`](Source::Karplus) excites a string and lets it ring, and
///   [`Noise`](Source::Noise) is every frequency at once, in whichever balance
///   its [`color`](NoiseColor) names. What the patch says is mostly what to
///   take *away*, downstream at the filter.
/// - **Bend one sine with another.** [`Fm2`](Source::Fm2) is one modulator on
///   one carrier; [`Fm4`](Source::Fm4) is four operators wired by a chosen
///   algorithm. What the patch says is a *depth*, and the spectrum is whatever
///   falls out of it. They are two sizes of one idea and both are kept — a
///   recipe that has outgrown the smaller one knows exactly what it wants next.
/// - **State the spectrum outright.** [`Additive`](Source::Additive) is a
///   table of partials, each placed, weighted and faded on its own. What the
///   patch says *is* the answer, with nothing left for a filter to arrive at.
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
    /// Raw noise — the head of every gunshot, impact and footstep, shaped by
    /// the filter and amp envelope.
    Noise {
        /// How its energy is spread across the spectrum. Absent means
        /// [`NoiseColor::White`], the hiss this source has always been.
        #[serde(default, skip_serializing_if = "is_white")]
        color: NoiseColor,
    },
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
    /// Four-operator FM: four sines wired by a chosen algorithm. Brass,
    /// basses, sustained pads, and bell-into-body layers — the FM territory
    /// two operators cannot reach, because two of them can only produce one
    /// modulator-carrier relationship.
    ///
    /// [`Source::Fm2`] is not superseded and is usually the right one: it is
    /// two numbers rather than a routing and four operators, and it covers
    /// bells, electric pianos and struck metal well. Reach for this when the
    /// sound needs a modulator with its *own* modulator, or two voices layered
    /// into one note — see [`Algorithm`] for the routings and [`Operator`] for
    /// what each of the four carries.
    Fm4 {
        /// How the four are wired. There is no default: the routing is the
        /// parameter that decides what kind of sound this is, and a silent one
        /// would make eight very different instruments look like one.
        algorithm: Algorithm,
        /// The four operators, in the order the algorithm diagrams number them
        /// — exactly [`FM_OPERATORS`] of them, so a list of three is refused as
        /// it is parsed rather than rendered with a stand-in.
        operators: [Operator; FM_OPERATORS],
        /// How much modulation depth a full-velocity strike adds to **every
        /// operator the algorithm uses as a modulator**.
        ///
        /// The [`Source::Fm2`] field of the same name, applied to a routing:
        /// in FM the index *is* the brightness, so this is the difference
        /// between a horn played softly and one leaned on. Carriers are left
        /// alone — a carrier's level is its share of the mix, and velocity
        /// already reaches the level through the amp envelope.
        ///
        /// Defaults to `0.0`. Added to each modulator's level and then floored
        /// at zero, for the reason [`Source::Fm2`]'s `vel_index` is: a
        /// negative index only mirrors the modulator and sounds exactly as
        /// bright as its positive twin, so an unclamped sum would make a
        /// darkening routing start brightening again the moment it crossed
        /// over.
        #[serde(default)]
        vel_index: f32,
    },
    /// An additive series: up to [`MAX_PARTIALS`] sine partials, each placed,
    /// weighted and faded on its own, summed into one tone. Organs, bowed and
    /// blown sustains, glass, and bells that are not FM-metallic.
    ///
    /// The one source here that states a spectrum instead of shaping one.
    Additive {
        /// The partials, summed and normalised by the sum of the gains of the
        /// ones that sound. Order carries no meaning — a series is a set of
        /// frequencies, not a chain — but it is what decides which partial a
        /// refusal is counted against.
        partials: Vec<Partial>,
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
            Self::Noise { .. } => "noise",
            Self::Fm2 { .. } => "fm2",
            Self::Fm4 { .. } => "fm4",
            Self::Additive { .. } => "additive",
        }
    }
}

/// An ADSR envelope: three timed segments and a level held between them.
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
    /// How the three timed segments bend on their way to their destination.
    /// `0.0` is the straight line every envelope was before this field
    /// existed; positive is the exponential approach a physical thing makes.
    ///
    /// Nothing physical decays in a straight line. A plucked string, a struck
    /// bell and a decaying room all lose energy in proportion to how much they
    /// still have, which is an exponential, and the ear is unusually good at
    /// telling the two apart: a linear decay holds up too long and then
    /// arrives at silence too abruptly, and that *sag* is one of the handful
    /// of cues that reliably says "synthesised" about an otherwise well-made
    /// sound.
    ///
    /// One number for the whole envelope rather than one per segment, because
    /// the same curve is the right one in all three places — attack, decay and
    /// release are each an approach to a destination, and an exponential
    /// approach means *fast at first, easing in*. On the way up that is a
    /// capacitor charging; on the way down it is the same capacitor
    /// discharging.
    ///
    /// Negative is legal and inverts the bend into a slow start and a sudden
    /// arrival — not a shape anything physical makes, but a deliberate and
    /// useful one for a swell. Magnitudes past what the renderer can evaluate
    /// without overflowing are clamped; the `env` module names the number.
    ///
    /// **Defaulting to linear is load-bearing, not timidity.** A bake is
    /// addressed by the hash of its recipe's bytes, so an envelope that
    /// started curving on its own — or a serialiser that started writing
    /// `"curve": 0.0` into every document — would invalidate every cached bake
    /// in every project at once. Changing this default later is therefore a
    /// deliberate one-time break, argued in its own pull request, never a
    /// quiet tweak.
    #[serde(default, skip_serializing_if = "is_linear")]
    pub curve: f32,
}

/// Whether an envelope bends at all — the test that keeps `"curve": 0.0` out
/// of every saved document, for the reason [`Adsr::curve`] gives.
fn is_linear(curve: &f32) -> bool {
    *curve == 0.0
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
            curve: 0.0,
        }
    }
}

/// An envelope aimed at the note's own pitch: a sweep that happens once and
/// settles.
///
/// This is the primitive an [`Lfo`] aimed at [`LfoTarget::Pitch`] is not. A
/// vibrato is cyclic — it wobbles around the played note for as long as the
/// note lasts. A great many instruments instead start *off* their pitch and
/// arrive on it exactly once: a kick drum starts near 90 Hz and is at 50 in
/// about 40 ms, and a tom, an 808, a timpani and a laser zap are the same move
/// at different speeds and depths. Without this the only way to write one is
/// to fake it through the filter, which approximates the brightness of the
/// gesture and none of the pitch.
///
/// It stacks additively with a vibrato rather than replacing it, the way
/// [`Filter::env_amount`] and [`Filter::vel_cutoff`] do: a patch may sweep down
/// onto its note and then wobble around it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchEnv {
    /// How many semitones the envelope adds at full level.
    ///
    /// The played note is where the sweep **ends**, not where it starts: under
    /// the usual shape — full immediately, decaying to nothing — the note
    /// begins `semitones` away and arrives on the pitch it was played at. So
    /// **positive falls onto the note from above**, which is what a kick, a
    /// tom and an 808 do, and negative rises onto it from below, which is a
    /// reverse zap. Both directions are one sign apart and neither is
    /// privileged; naming the destination is what makes a drum patch transpose
    /// like an instrument.
    ///
    /// Semitones rather than Hz because pitch is logarithmic: the same number
    /// of Hz is an octave low down and a rounding error high up, so a sweep
    /// written in Hz stops meaning the same gesture the moment the patch is
    /// played at another pitch. In semitones it transposes with the note.
    pub semitones: f32,
    /// The envelope doing the sweeping, with its own timings and curve.
    ///
    /// Defaults to [`Adsr::default`], which sustains — so a sweep worth
    /// hearing spells out a decay and a sustain of zero, the shape that says
    /// "move once, then settle".
    #[serde(default)]
    pub adsr: Adsr,
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

/// The most bands one EQ may carry.
///
/// Five kinds exist and a thorough treatment is rarely more than one of each —
/// a high-pass, a shelf at either end and a couple of peaks. Past that a
/// recipe is assembling a filter bank one band at a time, which is a different
/// tool from the one this is; the cap is the argument [`MAX_OSCS`] makes about
/// a stack, applied to arithmetic that runs over every sample of every note.
pub const MAX_EQ_BANDS: usize = 8;

/// What one EQ band does to the spectrum.
///
/// Five, and they are the five a mix is actually made of. Two remove an end of
/// the range outright and have no amount to ask for; three change how much of
/// a region there is, and so read their `gain_db`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqKind {
    /// Everything below the frequency goes — the first move any engineer makes
    /// on anything that is not the bass.
    HighPass,
    /// Everything below the frequency moves by `gain_db`, together.
    LowShelf,
    /// A bump or a dip centred on the frequency, `q` wide. Taking 250 Hz out
    /// of a pad and keeping the pad is this one.
    Peak,
    /// Everything above the frequency moves by `gain_db`, together. Air.
    HighShelf,
    /// Everything above the frequency goes.
    LowPass,
}

impl EqKind {
    /// Whether this kind reads `gain_db` at all.
    ///
    /// The two pass filters do not: they remove an end of the range, and there
    /// is no amount of *gone* to ask for. That distinction is what lets the
    /// zero-gain bypass be a rule rather than a special case — `gain_db: 0.0`
    /// means "this band does nothing" for the three that read it, and would
    /// silently disable the two that do not.
    pub fn takes_gain(self) -> bool {
        matches!(self, Self::LowShelf | Self::Peak | Self::HighShelf)
    }

    /// Where a band of this kind sits when the recipe does not say, in Hz.
    ///
    /// **The two numbers the bake report splits its bands at**, and that is
    /// the point rather than a coincidence. A report reading `low 61%` is a
    /// finding about the energy under 250 Hz, so the band that treats it
    /// should be reachable without the reader converting anything: the three
    /// kinds that work on the bottom default to the low crossover and the two
    /// that work on the top default to the high one. So
    /// `{ "kind": "low_shelf", "gain_db": -3 }` reads as *take 3 dB off the
    /// thing the report just called low*.
    pub fn crossover(self) -> f32 {
        match self {
            Self::HighPass | Self::LowShelf | Self::Peak => bands::LOW_HZ,
            Self::HighShelf | Self::LowPass => bands::HIGH_HZ,
        }
    }
}

/// One band of an [`Fx::Eq`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    /// What this band does to the spectrum.
    pub kind: EqKind,
    /// Where it acts, in Hz — a corner for the pass filters and the shelves, a
    /// centre for a peak.
    ///
    /// Absent means [`EqKind::crossover`]: the boundary the bake report
    /// already draws around the end of the range this kind works on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freq: Option<f32>,
    /// How much is added or taken away, in decibels — positive boosts,
    /// negative cuts.
    ///
    /// **`0.0` is a bypass.** A band at zero gain is not applied at all, so it
    /// is sample-identical to leaving it out, which is what lets a recipe list
    /// the bands it is thinking about and sweep one of them without the others
    /// colouring anything on the way past.
    ///
    /// Read only by the three kinds [`EqKind::takes_gain`] names.
    #[serde(default)]
    pub gain_db: f32,
    /// How narrow the band is; higher is narrower. Around `0.7` is the gentle,
    /// non-resonant default, `2` is a noticeable notch and `8` is a surgical
    /// one aimed at a single ringing frequency.
    ///
    /// It means the same thing for all five kinds: the pass filters and the
    /// shelves take it as the steepness of their corner, where much past `1`
    /// starts to put a resonant bump at the corner itself.
    #[serde(default = "gentle_q")]
    pub q: f32,
}

impl EqBand {
    /// Where this band acts, in Hz, with the default resolved.
    pub fn hz(&self) -> f32 {
        self.freq.unwrap_or_else(|| self.kind.crossover())
    }
}

/// One effect in the post-chain, applied in list order.
///
/// A limiter is *always* applied at bake and is deliberately not listed here —
/// it is not a choice the recipe gets to make.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// A compressor: the level control that acts on the loud moments and
    /// leaves the quiet ones, which is what makes a part *sit* rather than
    /// merely be at a volume.
    ///
    /// A limiter is not this at another setting: one is chosen and meant to be
    /// heard, the other is unconditional and is the promise that a bake cannot
    /// clip. The `fx::compress` module carries the whole argument.
    Compress {
        /// The level above which the signal is pushed down, in dBFS. Clamped
        /// to −60…0: full scale is where the limiter's job starts.
        threshold: f32,
        /// How much of each decibel over the threshold survives, as `n:1` —
        /// `2` is gentle, `4` is the workhorse, `10` is a part pinned in
        /// place. `1` is no compression at all, and is an exact bypass unless
        /// `makeup` is asking for something. Clamped at 20:1, past which this
        /// would be a limiter.
        ratio: f32,
        /// Seconds the gain takes to arrive at full reduction. Because this
        /// renders offline the duck is *ready* when the peak lands rather than
        /// chasing it, which is a more transparent attack than a hardware
        /// compressor's and lets a transient through less; `mix` is how a
        /// recipe keeps one.
        #[serde(default = "attack_default")]
        attack: f32,
        /// Seconds the gain takes to recover afterwards. Too short and the
        /// mix breathes audibly between hits; a tenth of a second upwards is
        /// the usual range.
        #[serde(default = "release_default")]
        release: f32,
        /// Decibels handed back to the compressed copy, to replace what the
        /// reduction took. Clamped to ±24.
        #[serde(default)]
        makeup: f32,
        /// Wet/dry blend, `0..=1`. Below 1 this is **parallel** ("New York")
        /// compression: a squashed copy under an untouched one, which adds
        /// density without flattening the transients. `0.0` is an exact
        /// bypass.
        #[serde(default = "one")]
        mix: f32,
        /// Another track, by name, whose part the detector listens to instead
        /// of this one's — the kick pressing the bass down on every beat.
        ///
        /// **A track chain only.** A patch chain runs per note and the song's
        /// runs on the sum; neither sits anywhere a track name can be read, so
        /// both refuse this rather than ignoring it. The part handed over is
        /// the key track's *as played* — before its own chain, its `gain` and
        /// its `pan` — so `crate::song`'s mixer has the reasoning and the
        /// consequence: two tracks may key each other, because neither is
        /// waiting on the other's output.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sidechain: Option<String>,
    },
    /// Several detuned, delayed copies of the signal, spread across the
    /// stereo field: the difference between one synthesiser playing a note and
    /// a section playing it. The only effect here that makes one source sound
    /// like more than one thing.
    Chorus {
        /// How fast each copy's delay sweeps, in Hz. `0.3`–`1.5` is the usual
        /// range; the voices are spread slightly either side of it so the
        /// ensemble does not breathe in unison. Clamped at 10, past which the
        /// modulation sidebands stop being a detune.
        rate: f32,
        /// How far the sweep moves, `0..=1` — how far apart the copies are
        /// pushed in pitch. Around `0.3` is a shimmer, `0.8` is a wide
        /// ensemble, and `1.0` starts to sound seasick on a sustained note.
        depth: f32,
        /// How many copies, clamped to 2–4. Two is a double-tracked part, four
        /// is a section; past four each new voice sits on top of one already
        /// there. It is a thickness control and not a fader — the copies are
        /// normalised, so the wet signal is the same level whatever this says.
        #[serde(default = "voices_default")]
        voices: usize,
        /// Wet/dry blend, `0..=1`. The dry signal stays exactly where it was
        /// and the copies arrive around it, so this is how far the source
        /// spreads rather than how loud the effect is.
        mix: f32,
    },
    /// A small stack of filter bands: the treatment for what the bake report
    /// diagnoses. A gain fader answers a muddy pad by removing the pad; this
    /// answers it by removing 250 Hz and keeping the rest.
    Eq {
        /// The bands, applied in list order, at most [`MAX_EQ_BANDS`] of them.
        /// Order between them barely matters — filters commute to within
        /// rounding — but a list is a list, and it is honoured as written.
        bands: Vec<EqBand>,
    },
}

fn one() -> f32 {
    1.0
}

/// Whether a noise source is the plain hiss — the test that keeps
/// `"color": "white"` out of every saved document, for the reason
/// [`Adsr::curve`] gives about defaults and cached bakes.
fn is_white(color: &NoiseColor) -> bool {
    matches!(color, NoiseColor::White)
}

/// One oscillator, undoubled: what an `Osc` meant before unison existed.
fn solo() -> usize {
    1
}

/// Whether an oscillator is a single voice — the test that keeps
/// `"voices": 1` out of every saved document, for the reason [`is_white`]
/// gives.
fn is_solo(voices: &usize) -> bool {
    *voices == 1
}

/// How far apart unison voices sit when the recipe does not say: 12 cents
/// end to end, an eighth of a semitone, which is a shimmer rather than a
/// chord.
fn spread_default() -> f32 {
    12.0
}

/// Whether a spread is the written-nothing one, so it is not saved either.
fn is_default_spread(spread: &f32) -> bool {
    *spread == spread_default()
}

fn gentle_q() -> f32 {
    0.707
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

/// A compressor's attack when the recipe does not say: 10 ms, fast enough to
/// catch a hit and slow enough not to be shaping the waveform.
fn attack_default() -> f32 {
    0.01
}

/// A compressor's release when the recipe does not say: 150 ms, roughly one
/// beat at a mid tempo, which is where a sidechain duck sounds like the track
/// rather than like an effect.
fn release_default() -> f32 {
    0.15
}

fn voices_default() -> usize {
    3
}
