//! four-operator FM: four sines, wired by the algorithm the recipe picked.
//!
//! [`two`](super::two) is one modulator on one carrier, which is one
//! relationship and so one kind of sound. Here the routing is chosen from
//! [`Algorithm`]'s table, and that choice is what opens the rest of FM: a
//! modulator with its own modulator, two modulators summed onto one carrier,
//! two whole voices mixed into one note.
//!
//! ## One pass, in order, and why that is the whole termination argument
//!
//! Every edge in the table runs from a lower-numbered operator to a
//! higher-numbered one, so a sample is produced by walking operators 1, 2, 3, 4
//! once: whatever modulates an operator was computed earlier in the same pass.
//! There is no iteration to converge and no graph to traverse — the routing
//! being a table rather than an edge list is what buys that, and
//! [`patch::fm`](crate::patch::fm) carries the argument for why it is a table.
//!
//! ## What an operator's level means
//!
//! A **modulator's** level is an index in radians, added into its target's
//! phase — the same quantity [`two`](super::two) calls `index`. A **carrier's**
//! level is a weight in the mix, and the carriers are normalised by their
//! total, so adding one thickens the tone rather than making it louder, the
//! rule an oscillator stack already follows. That
//! normalisation also bounds the output: the weights sum to one and each
//! carrier is a sine, so the worst case is every carrier peaking together at
//! exactly ±1.
//!
//! No operator is ever both under any row of the table, so no level ever means
//! two things at once.
//!
//! ## Feedback
//!
//! An operator may bend its own phase with its own last two output samples,
//! which is what reaches the rasping, growling end of the FM range. It is the
//! one place in the signal path where an output is read back as an input, so
//! the bound that keeps it from being the free-graph loop a patch may not
//! contain is stated where it lives, in [`feedback`](super::feedback). The
//! state it needs — two past samples per operator — is kept here, because it
//! belongs to the pass rather than to the arithmetic.
//!
//! ## Above Nyquist: an operator is dropped, sidebands are not
//!
//! An operator whose own frequency is at or past half the sample rate is not
//! rendered at all. Its sine cannot be represented, so what it would
//! contribute is not a bright partial but an aliased one at some unrelated
//! frequency — as a carrier it would be an audible wrong note, and as a
//! modulator it would bend its target at a ratio nobody wrote. That question
//! is the additive source's too, so it is asked in one place —
//! [`nyquist`](crate::core::nyquist) — which also carries why it is decided
//! once per note against the top of the pitch track. Carriers that are dropped
//! leave the normalisation too, so a note played high does not also get
//! quieter.
//!
//! **The sidebands are a different matter and are not fixed here.** FM does not
//! synthesise its sidebands separately — they are what falls out of one `sin`
//! of a bent phase — so there is nothing to drop, and the ones above Nyquist
//! fold back into the audible band. Removing them would mean rendering the
//! source at several times the rate and filtering on the way down, which is a
//! cost every note would pay for a case a recipe can simply avoid. The bound
//! to write against is Carson's: a modulator of frequency `m` at index `i`
//! spreads a carrier roughly `(i + 1) × m` either side of itself, so keeping
//! `pitch × ratio × (index + 1)` under about 20 kHz keeps the fold-back
//! inaudible. High ratios and high indices belong on low notes, which is also
//! where they sound like anything.

use std::f32::consts::TAU;

use super::{feedback, voicing};
use crate::core::{env, nyquist};
use crate::patch::{Algorithm, FM_OPERATORS, Operator};

/// Render four-operator FM into `out`, following the per-sample frequency
/// track `freqs`.
///
/// `levels` is each operator's level **with velocity already resolved into
/// it** — the caller does that, because how a velocity becomes an index is
/// bookkeeping and this module is the algorithm. `gate` is how long the note is
/// held, which is what the per-operator envelopes are driven by.
pub(crate) fn render(
    out: &mut [f32],
    freqs: &[f32],
    algorithm: Algorithm,
    operators: &[Operator; FM_OPERATORS],
    levels: &[f32; FM_OPERATORS],
    gate: f32,
    rate: f32,
) {
    let ceiling = nyquist::ratio_ceiling(freqs, rate);
    let sounding = voicing::sounding(operators, ceiling);
    let norm = voicing::normaliser(algorithm, levels, &sounding);
    let mut phase = [0.0f32; FM_OPERATORS];
    let mut fed = [[0.0f32; 2]; FM_OPERATORS];
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / rate;
        let base = freqs.get(i).copied().unwrap_or(0.0);
        let mut shaped = [0.0f32; FM_OPERATORS];
        let mut mix = 0.0;
        for op in 0..FM_OPERATORS {
            if !sounding[op] {
                continue;
            }
            let operator = &operators[op];
            let bend = modulation(algorithm.modulators(op), levels, &shaped)
                + feedback::bend(operator.feedback, fed[op]);
            shaped[op] = envelope(operator, t, gate) * (TAU * phase[op] + bend).sin();
            fed[op] = [shaped[op], fed[op][0]];
            if algorithm.is_carrier(op) {
                mix += levels[op].max(0.0) * shaped[op];
            }
            phase[op] = (phase[op] + base * operator.ratio / rate).fract();
        }
        *s = mix * norm;
    }
}

/// How far this operator's modulators bend its phase, in radians: each one's
/// level times its already-shaped output.
///
/// `mask` only ever names operators below this one, so every value it reads
/// out of `shaped` was written earlier in the same pass.
fn modulation(mask: u8, levels: &[f32; FM_OPERATORS], shaped: &[f32; FM_OPERATORS]) -> f32 {
    (0..FM_OPERATORS)
        .filter(|op| mask & (1 << op) != 0)
        .map(|op| levels[op] * shaped[op])
        .sum()
}

/// An operator's own envelope at time `t`, or a flat full level for an operator
/// that does not carry one.
#[inline]
fn envelope(operator: &Operator, t: f32, gate: f32) -> f32 {
    match &operator.env {
        Some(adsr) => env::level_at(adsr, t, gate),
        None => 1.0,
    }
}

/// The routings by what they put in the spectrum, the feedback by its bound,
/// and the envelopes by the fact that they are *per operator*.
///
/// Read as levels out of a one-bin DFT, with one window and one fundamental
/// chosen so that every frequency asserted lands exactly on a bin: 100 Hz over
/// 4410 samples, a tenth of a second, in which partial `k` completes exactly
/// `10k` cycles. There is no leakage for a wrong number to hide in.
///
/// A magnitude is blind to a sign flip and to a phase running backwards, so a
/// file of nothing but spectral assertions would pass with the whole buffer
/// negated or every oscillator running in reverse.
/// [`the_carrier_is_added_in_and_walks_forward`] closes both against
/// hand-computed samples instead.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::Adsr;

    /// The played pitch every test below measures from.
    const BASE: f32 = 100.0;

    /// The rate everything renders at, as the DSP sees it.
    const RATE: f32 = 44_100.0;

    /// A tenth of a second: a whole number of cycles of [`BASE`] and of every
    /// harmonic of it.
    const WINDOW: usize = 4410;

    /// A gate longer than any window here, so no operator envelope is in its
    /// release unless a test puts it there.
    const HELD: f32 = 10.0;

    fn op(ratio: f32, level: f32) -> Operator {
        Operator {
            ratio,
            level,
            feedback: 0.0,
            env: None,
        }
    }

    /// `operators` rendered under `algorithm` at `hz`, over `n` samples, with
    /// each operator's level taken as written.
    fn voice(algorithm: Algorithm, operators: [Operator; 4], hz: f32, n: usize) -> Vec<f32> {
        let levels = std::array::from_fn(|i| operators[i].level);
        let mut out = vec![0.0; n];
        render(
            &mut out,
            &vec![hz; n],
            algorithm,
            &operators,
            &levels,
            HELD,
            RATE,
        );
        out
    }

    /// The amplitude of the sinusoid at `hz` in `buf`, by a one-bin DFT.
    fn level_at(buf: &[f32], hz: f32) -> f32 {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, s) in buf.iter().enumerate() {
            let phase = std::f64::consts::TAU * f64::from(hz) * i as f64 / f64::from(RATE);
            re += f64::from(*s) * phase.cos();
            im -= f64::from(*s) * phase.sin();
        }
        (2.0 * re.hypot(im) / buf.len() as f64) as f32
    }

    /// Which operators are heard, read out of the spectrum rather than off the
    /// table: two carriers at different ratios put energy at two frequencies,
    /// and a routing that hears only the last of them puts energy at one.
    ///
    /// Operator 3's level is zero in both, so operator 4 is an unmodulated sine
    /// either way and the only difference between the two spectra is whether
    /// operator 2 reached the output at all.
    #[test]
    fn a_carrier_is_heard_and_a_modulator_is_not() {
        let operators = [op(2.0, 0.0), op(1.0, 1.0), op(5.0, 0.0), op(3.0, 1.0)];
        let twin = voice(Algorithm::Twin, operators, BASE, WINDOW);
        assert!(
            (level_at(&twin, BASE) - 0.5).abs() < 1e-3,
            "operator 2 is a carrier under twin, at half the mix"
        );
        assert!((level_at(&twin, BASE * 3.0) - 0.5).abs() < 1e-3);
        let chain = voice(Algorithm::Chain, operators, BASE, WINDOW);
        assert!(
            level_at(&chain, BASE) < 1e-3,
            "under chain only operator 4 is heard"
        );
        assert!(
            (level_at(&chain, BASE * 3.0) - 1.0).abs() < 1e-3,
            "and it has the whole mix to itself"
        );
    }

    /// A sideband that only one routing produces. Operator 1 modulates all
    /// three carriers under `fan` and only operator 2 under `pair_and_two`, so
    /// with operator 3 at ratio 5 the two routings differ at 4×: a sideband
    /// under one, silence under the other.
    #[test]
    fn one_modulator_reaches_three_carriers_only_under_fan() {
        // Three ratios whose sideband combs never meet: operator 2 puts energy
        // at whole multiples of the pitch, operator 3 at halves and operator 4
        // at quarters. Two carriers bent by one modulator would otherwise land
        // sidebands on each other and could cancel, which is a fact about FM
        // rather than about the routing under test. The window is doubled so a
        // quarter-multiple still falls exactly on a bin.
        let operators = [op(1.0, 3.0), op(1.0, 1.0), op(5.5, 1.0), op(9.25, 1.0)];
        let fan = voice(Algorithm::Fan, operators, BASE, WINDOW * 2);
        assert!(
            level_at(&fan, BASE * 4.5) > 0.05,
            "operator 3 is bent, so 5.5× − 1× is in the spectrum"
        );
        assert!(level_at(&fan, BASE * 8.25) > 0.05, "and so is 9.25× − 1×");
        let pair = voice(Algorithm::PairAndTwo, operators, BASE, WINDOW * 2);
        assert!(
            level_at(&pair, BASE * 4.5) < 1e-3,
            "under pair_and_two operators 3 and 4 are plain sines"
        );
        assert!(level_at(&pair, BASE * 8.25) < 1e-3);
        assert!(
            (level_at(&pair, BASE * 5.5) - 1.0 / 3.0).abs() < 1e-3,
            "and operator 3 is at its own ratio, at a third of the mix"
        );
    }

    /// `branch` and `fork` are the same shape numbered differently, which is a
    /// claim about *where a recipe writes its numbers* — so one operator list
    /// has to sound different under the two. Operator 2 at level 6 is a
    /// dead-end modulator of operator 3 under `branch` (whose own level is
    /// zero, so nothing reaches the carrier) and a modulator of the carrier
    /// itself under `fork`.
    #[test]
    fn the_same_operators_route_differently_under_branch_and_fork() {
        // The carrier sits at 8× so that every sideband it could grow stays
        // above zero Hz: a sideband that folded around DC would land back on
        // one of its own siblings and could cancel it, which is a fact about
        // FM rather than about the routing under test.
        let operators = [op(3.0, 0.0), op(1.0, 3.0), op(7.0, 0.0), op(8.0, 1.0)];
        let branch = voice(Algorithm::Branch, operators, BASE, WINDOW);
        assert!(
            (level_at(&branch, BASE * 8.0) - 1.0).abs() < 1e-3,
            "under branch the carrier is a bare sine"
        );
        assert!(level_at(&branch, BASE * 7.0) < 1e-3, "with no sidebands");
        assert!(level_at(&branch, BASE * 9.0) < 1e-3);
        let fork = voice(Algorithm::Fork, operators, BASE, WINDOW);
        assert!(
            level_at(&fork, BASE * 7.0) > 0.2,
            "under fork operator 2 bends the carrier, so 8× − 1× is there"
        );
        assert!(level_at(&fork, BASE * 9.0) > 0.2, "and 8× + 1×");
    }

    /// Depth stacks through a chain: under `stack` operator 1 bends operator 3
    /// directly, and under `chain` it reaches the carrier only through operator
    /// 2 — whose level is zero here, so the carrier comes out clean.
    #[test]
    fn a_chain_carries_depth_only_through_the_operator_between() {
        let operators = [op(1.0, 5.0), op(1.0, 0.0), op(1.0, 5.0), op(1.0, 1.0)];
        let chain = voice(Algorithm::Chain, operators, BASE, WINDOW);
        let stack = voice(Algorithm::Stack, operators, BASE, WINDOW);
        assert!(
            spread(&stack) > spread(&chain) * 1.5,
            "stack {} should out-spread chain {}",
            spread(&stack),
            spread(&chain)
        );
    }

    /// How much of the buffer's energy sits above the fourth harmonic — a
    /// brightness measure that a level change alone cannot move, since it is a
    /// share rather than an amount.
    fn spread(buf: &[f32]) -> f32 {
        let high: f32 = (5..20).map(|k| level_at(buf, BASE * k as f32)).sum();
        let all: f32 = (1..20).map(|k| level_at(buf, BASE * k as f32)).sum();
        high / all
    }

    /// The two things a magnitude spectrum cannot see: that a modulator is
    /// **added** into the carrier's phase, and that a phase walks **forward**.
    ///
    /// One carrier at ratio 1 bent by one modulator at ratio 1 and index 2, so
    /// the buffer is `sin(x + 2 sin x)` with `x = 2π × 100 t` — both phases
    /// start at zero, and the normalisation is 1 because the lone carrier is
    /// the only one at any level. The literals below are worked out from that
    /// formula by hand; recomputing the renderer's expression here would only
    /// assert that the code agrees with itself.
    ///
    /// - **Sample 0** is `sin(0)`, which every variant agrees on — it is here
    ///   to state that the phases start where the module says they do.
    /// - **Sample 40** is `x = 0.5699`, where the three readings are furthest
    ///   apart: `sin(0.5699 + 1.0795) = 0.9969` forward, `sin(0.5699 − 1.0795)
    ///   = −0.4875` if the modulator were subtracted, and `−0.9969` if the
    ///   phase ran backwards. No two of those are within a tolerance of each
    ///   other.
    #[test]
    fn the_carrier_is_added_in_and_walks_forward() {
        let buf = voice(
            Algorithm::Twin,
            [op(1.0, 2.0), op(1.0, 1.0), op(1.0, 0.0), op(1.0, 0.0)],
            BASE,
            512,
        );
        assert_eq!(buf[0], 0.0, "both phases start at zero, so sin(0)");
        assert!(
            (buf[40] - 0.996_94).abs() < 1e-4,
            "sample 40 read {}, where subtracting reads −0.4875 and a \
             backwards phase −0.9969",
            buf[40]
        );
    }

    /// A note rendered with the feedback path driven as hard as a document can
    /// ask for still stays inside unity and still makes a sound — the bound
    /// [`feedback`](super::feedback) proves arithmetically, seen through a
    /// whole buffer.
    #[test]
    fn feedback_is_bounded_however_it_is_written() {
        let mut driven = op(1.0, 1.0);
        driven.feedback = 1e9;
        let buf = voice(Algorithm::Parallel, [driven; 4], BASE, WINDOW);
        assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
        assert!(buf.iter().any(|s| s.abs() > 0.5), "and it is not silence");
    }

    /// The two bends an operator takes — from its modulators and from its own
    /// feedback — are **summed**, not differenced, and only a signed sample can
    /// say so. Every spectral assertion in this file reads a magnitude, and a
    /// magnitude is blind to the sign of one term inside a phase; the two
    /// tests above each hold one of the terms at zero, so neither notices
    /// either.
    ///
    /// One carrier at ratio 1 with feedback at full, under one modulator at
    /// ratio 1 and index 2, played at an eighth of the sample rate so a sample
    /// is an eighth of a cycle and the recursion is short enough to work by
    /// hand. Sample 0 is `sin(0)`; sample 1 has nothing fed back yet, so it is
    /// `sin(π/4 + 2 sin(π/4)) = 0.8087` either way. **Sample 2 is where they
    /// part**: the fed-back average is `π × 0.8087 / 2 = 1.2700`, so the
    /// forward sum reads `sin(π/2 + 2 + 1.2700) = −0.9917` and a difference
    /// would read `sin(π/2 + 2 − 1.2700) = +0.7454`. Opposite signs, and more
    /// than 1.7 apart.
    #[test]
    fn the_modulators_and_the_feedback_add_rather_than_cancel() {
        let mut carrier = op(1.0, 1.0);
        carrier.feedback = 1.0;
        // An eighth of the sample rate: 5512.5 Hz, whose ratio ceiling is
        // exactly 4, so every operator here is comfortably inside it.
        let buf = voice(
            Algorithm::Twin,
            [op(1.0, 2.0), carrier, op(1.0, 0.0), op(1.0, 0.0)],
            RATE / 8.0,
            8,
        );
        assert_eq!(buf[0], 0.0, "both phases start at zero");
        assert!(
            (buf[1] - 0.808_725).abs() < 1e-4,
            "sample 1 read {}",
            buf[1]
        );
        assert!(
            (buf[2] + 0.991_723).abs() < 1e-4,
            "sample 2 read {}, where a difference reads +0.7454",
            buf[2]
        );
    }

    /// And it is audible: a fed-back operator is no longer a sine, so harmonics
    /// appear that a clean one has none of.
    #[test]
    fn feedback_turns_a_sine_into_something_with_harmonics() {
        let clean = voice(Algorithm::Parallel, [op(1.0, 1.0); 4], BASE, WINDOW);
        assert!(level_at(&clean, BASE * 2.0) < 1e-3, "a sine has no second");
        let mut rough = op(1.0, 1.0);
        rough.feedback = 0.9;
        let buf = voice(Algorithm::Parallel, [rough; 4], BASE, WINDOW);
        assert!(level_at(&buf, BASE * 2.0) > 0.05, "the fed-back one does");
        assert!(level_at(&buf, BASE * 3.0) > 0.05);
    }

    /// The envelopes belong to the operators, not to the source: two carriers
    /// under one algorithm, one decaying and one held, and the spectrum says
    /// which is which as the note runs on.
    #[test]
    fn each_operator_follows_its_own_envelope() {
        let fading = Operator {
            env: Some(Adsr {
                a: 0.0,
                d: 0.25,
                s: 0.0,
                r: 0.0,
                curve: 0.0,
            }),
            ..op(1.0, 1.0)
        };
        let buf = voice(
            Algorithm::Twin,
            [op(1.0, 0.0), fading, op(1.0, 0.0), op(3.0, 1.0)],
            BASE,
            WINDOW * 4,
        );
        let (early, late) = (&buf[..WINDOW], &buf[WINDOW * 3..]);
        assert!(level_at(early, BASE) > 0.3, "the fading carrier starts up");
        assert!(level_at(late, BASE) < 0.01, "and has gone by the end");
        let held = (level_at(early, BASE * 3.0), level_at(late, BASE * 3.0));
        assert!(
            (held.0 - held.1).abs() < 0.02,
            "while the held carrier stays where it was: {held:?}"
        );
    }

    /// An operator past Nyquist is absent rather than folded. Operator 2 at
    /// ratio 30 on a 2 kHz note sits at 60 kHz and would alias to 15.9 kHz;
    /// nothing is there, and operator 1 has the whole mix because a dropped
    /// carrier leaves the normalisation too.
    #[test]
    fn an_operator_above_nyquist_is_dropped_rather_than_folded() {
        let high = 2000.0;
        let buf = voice(
            Algorithm::Parallel,
            [op(1.0, 1.0), op(30.0, 1.0), op(1.0, 0.0), op(1.0, 0.0)],
            high,
            WINDOW,
        );
        assert!(level_at(&buf, 15_900.0) < 1e-3, "no folded operator");
        assert!(
            (level_at(&buf, high) - 1.0).abs() < 1e-3,
            "and the one that sounds is not quieter for it"
        );
    }

    /// The routing decides which operators are dropped from the normalisation,
    /// so a modulator past Nyquist takes its bend away and leaves the level
    /// alone.
    #[test]
    fn a_modulator_above_nyquist_stops_bending_its_carrier() {
        let high = 2000.0;
        let buf = voice(
            Algorithm::Twin,
            [op(40.0, 6.0), op(1.0, 1.0), op(1.0, 0.0), op(1.0, 0.0)],
            high,
            WINDOW,
        );
        assert!(
            (level_at(&buf, high) - 1.0).abs() < 1e-3,
            "a bare carrier at full level"
        );
    }

    /// Whatever the routing and whatever the levels, the sum cannot leave
    /// `−1..=1`, and none of the eight is silent.
    #[test]
    fn every_algorithm_stays_inside_unity_and_makes_a_sound() {
        for algorithm in Algorithm::ALL {
            let buf = voice(
                algorithm,
                [op(1.0, 3.0), op(2.0, 4.0), op(3.0, 2.0), op(1.0, 1.0)],
                BASE,
                WINDOW,
            );
            let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak <= 1.0, "{algorithm:?} peaked at {peak}");
            assert!(peak > 0.2, "{algorithm:?} is inaudible ({peak})");
        }
    }
}
