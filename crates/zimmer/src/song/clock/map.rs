//! The arithmetic of a tempo that moves: beats to seconds and back through a
//! song's [`TempoChange`]s.
//!
//! The map is a run of **segments**, one per stretch between two points: each
//! either holds a tempo or moves it evenly per beat to the next point's. Where
//! a beat falls is the integral of seconds-per-beat up to it, and both kinds of
//! segment have that integral in closed form — so a position is a lookup and
//! one formula, never a sum walked beat by beat, and the renderer asking the
//! same question a million times over a fader gets a million exact answers
//! rather than a million accumulated errors.
//!
//! For a segment starting at tempo `B` and gaining `s` bpm per beat, the tempo
//! `d` beats in is `B + s·d`, and the time taken to get there is
//!
//! ```text
//! t(d) = ∫₀ᵈ 60 / (B + s·x) dx = (60 / s) · ln(1 + s·d / B)
//! d(t) = (B / s) · (exp(s·t / 60) − 1)
//! ```
//!
//! which is `60·d / B` in the limit `s → 0`, the held case, and is written
//! with `ln_1p` and `exp_m1` so a gentle ramp does not lose its precision to
//! the `1 +`. Everything is `f64` inside: a piece is minutes long and a sample
//! is 23 µs, and `f32` has no room for both in one number.
//!
//! **The map repeats with the arrangement.** Under a `loop` fit a song plays
//! its arrangement several times over, and each pass is the piece as written —
//! so an accelerando speeds up every time round rather than once, leaving every
//! pass after the first stuck at the final tempo. That is also what keeps the
//! length arithmetic honest: `fit` counts passes of one length, and a map that
//! carried on would make the second pass a different length from the first.

use super::super::tempo::TempoChange;

/// A song's tempo as a function of its beats, over as many passes as it plays.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Map {
    /// In beat order, the first starting on beat 0. Never empty: the last one
    /// holds the final tempo for as long as anything asks.
    segments: Vec<Segment>,
    /// Beats in one pass of the arrangement.
    period: f64,
    /// Seconds one pass lasts, the map applied.
    pass: f64,
    /// How many passes are played — the last one runs on past its end, so a
    /// note ringing past the final beat is placed at the final tempo.
    passes: u32,
}

/// One stretch of the map: a tempo held, or one moving evenly per beat.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Segment {
    /// The beat it begins on, counted within a pass.
    from: f64,
    /// Seconds into the pass that beat falls at.
    at: f64,
    /// The tempo on its first beat.
    bpm: f64,
    /// Beats per minute gained per beat; zero holds the tempo.
    slope: f64,
}

impl Segment {
    /// Seconds from this segment's first beat to `beats` past it.
    fn seconds(self, beats: f64) -> f64 {
        if self.slope == 0.0 {
            beats * 60.0 / self.bpm
        } else {
            60.0 / self.slope * (self.slope * beats / self.bpm).ln_1p()
        }
    }

    /// Beats from this segment's first beat to `seconds` past it.
    fn beats(self, seconds: f64) -> f64 {
        if self.slope == 0.0 {
            seconds * self.bpm / 60.0
        } else {
            self.bpm / self.slope * (self.slope * seconds / 60.0).exp_m1()
        }
    }
}

impl Map {
    /// The map of a song starting at `bpm` and changing as `changes` say, every
    /// tempo multiplied by `scale` — which is how a `stretch` fit moves the
    /// whole piece faster or slower without reshaping it.
    ///
    /// `changes` is taken as validated: beats after zero and ascending, every
    /// tempo positive.
    pub(super) fn new(
        bpm: f32,
        changes: &[TempoChange],
        scale: f64,
        period: f32,
        passes: u32,
    ) -> Self {
        let mut segments = Vec::with_capacity(changes.len() + 1);
        let (mut from, mut at, mut tempo) = (0.0, 0.0, f64::from(bpm) * scale);
        for change in changes {
            let beat = f64::from(change.beat);
            let target = f64::from(change.bpm) * scale;
            let slope = if change.ramp {
                (target - tempo) / (beat - from)
            } else {
                0.0
            };
            let segment = Segment {
                from,
                at,
                bpm: tempo,
                slope,
            };
            at += segment.seconds(beat - from);
            segments.push(segment);
            (from, tempo) = (beat, target);
        }
        segments.push(Segment {
            from,
            at,
            bpm: tempo,
            slope: 0.0,
        });
        let mut map = Self {
            segments,
            period: f64::from(period),
            pass: 0.0,
            passes: passes.max(1),
        };
        map.pass = map.seconds_within(map.period);
        map
    }

    /// Seconds into the piece that `beats` from its start falls at.
    pub(super) fn seconds(&self, beats: f64) -> f64 {
        let pass = self.pass_of(beats / self.period);
        pass * self.pass + self.seconds_within(beats - pass * self.period)
    }

    /// Beats from the start of the piece at `seconds` into it.
    pub(super) fn beats(&self, seconds: f64) -> f64 {
        let pass = self.pass_of(seconds / self.pass);
        pass * self.period + self.beats_within(seconds - pass * self.pass)
    }

    /// Which pass a position falls in, given as a count of whole passes — the
    /// last one keeping everything past it, so the ring-out after the final
    /// beat carries on at the final tempo rather than starting another pass.
    fn pass_of(&self, passes: f64) -> f64 {
        if passes.is_finite() {
            passes.floor().clamp(0.0, f64::from(self.passes - 1))
        } else {
            0.0
        }
    }

    /// Seconds into one pass at `beat` of it.
    fn seconds_within(&self, beat: f64) -> f64 {
        let beat = beat.max(0.0);
        let segment = self.segment(self.segments.partition_point(|it| it.from <= beat));
        segment.at + segment.seconds(beat - segment.from)
    }

    /// Beats into one pass at `seconds` of it.
    fn beats_within(&self, seconds: f64) -> f64 {
        let seconds = seconds.max(0.0);
        let segment = self.segment(self.segments.partition_point(|it| it.at <= seconds));
        segment.from + segment.beats(seconds - segment.at)
    }

    /// The segment before the `after`-th — the one a position that sorts at
    /// `after` sits in. The first segment starts at zero, so every position
    /// from zero on has one.
    fn segment(&self, after: usize) -> Segment {
        self.segments[after.saturating_sub(1)]
    }
}

/// The ramp maths, held to the integral it claims to be — worked here in
/// closed form by hand, and separately by summing it in small steps, so the
/// test does not share a formula with the code.
#[cfg(test)]
mod tests {
    use super::*;

    /// 120 bpm, rising evenly to 180 by beat 16 and holding there, over one
    /// pass of 32 beats.
    fn accelerando() -> Map {
        Map::new(120.0, &[TempoChange::ramp(16.0, 180.0)], 1.0, 32.0, 1)
    }

    /// Seconds to `beats` into [`accelerando`], by summing seconds-per-beat
    /// over a hundred thousand slices of each beat — the integral done the
    /// slow way.
    fn summed(beats: f64) -> f64 {
        let steps = (beats * 100_000.0) as usize;
        let width = beats / steps as f64;
        (0..steps)
            .map(|step| {
                let beat = (step as f64 + 0.5) * width;
                let bpm = if beat < 16.0 {
                    120.0 + 60.0 * beat / 16.0
                } else {
                    180.0
                };
                60.0 / bpm * width
            })
            .sum()
    }

    #[test]
    fn a_ramp_places_its_beats_where_the_integral_says() {
        let map = accelerando();
        // (16 · 60 / 60) · ln(180 / 120): the whole ramp, in closed form.
        let ramp = 16.0 * (1.5f64).ln();
        assert!((map.seconds(16.0) - ramp).abs() < 1e-9);
        for beats in [0.5, 3.0, 8.0, 15.9, 16.0, 20.0, 31.0] {
            let got = map.seconds(beats);
            assert!((got - summed(beats)).abs() < 1e-6, "beat {beats}: {got}");
        }
        // Past the ramp the tempo holds at 180: a third of a second a beat.
        assert!((map.seconds(22.0) - (ramp + 2.0)).abs() < 1e-9);
    }

    #[test]
    fn seconds_to_beats_undoes_beats_to_seconds() {
        let map = accelerando();
        for beats in [0.0, 1.0, 7.25, 16.0, 16.5, 40.0] {
            assert!((map.beats(map.seconds(beats)) - beats).abs() < 1e-9);
        }
    }

    #[test]
    fn a_jump_holds_the_old_tempo_up_to_the_beat_it_is_written_on() {
        let map = Map::new(120.0, &[TempoChange::jump(4.0, 60.0)], 1.0, 8.0, 1);
        assert_eq!(map.seconds(4.0), 2.0, "four beats at 120");
        assert_eq!(map.seconds(6.0), 4.0, "then a second a beat");
    }

    #[test]
    fn a_ramp_down_is_the_same_integral_the_other_way() {
        let map = Map::new(120.0, &[TempoChange::ramp(8.0, 60.0)], 1.0, 8.0, 1);
        // (8 · 60 / −60) · ln(60 / 120) = 8 · ln 2.
        assert!((map.seconds(8.0) - 8.0 * 2f64.ln()).abs() < 1e-9);
    }

    #[test]
    fn each_pass_plays_the_map_again_and_the_last_runs_on() {
        let map = Map::new(120.0, &[TempoChange::jump(2.0, 60.0)], 1.0, 4.0, 2);
        // One pass: two beats at half a second, two at a second.
        assert_eq!(map.seconds(4.0), 3.0);
        assert_eq!(map.seconds(5.0), 3.5, "the second pass starts back at 120");
        assert_eq!(map.seconds(8.0), 6.0);
        assert_eq!(map.seconds(9.0), 7.0, "past the end, the final tempo holds");
    }

    #[test]
    fn scaling_the_map_scales_every_second_in_it() {
        let written = accelerando();
        let faster = Map::new(120.0, &[TempoChange::ramp(16.0, 180.0)], 1.25, 32.0, 1);
        for beats in [3.0, 16.0, 30.0] {
            assert!((faster.seconds(beats) * 1.25 - written.seconds(beats)).abs() < 1e-9);
        }
    }
}
