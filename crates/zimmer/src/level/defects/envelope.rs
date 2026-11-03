//! Dropouts and stepped gain, both read off a channel's energy every half
//! millisecond.
//!
//! **A dropout** is a run of blocks [`DEPTH_DB`] under the sound on both sides
//! of it, no longer than [`LONGEST_DROPOUT`]: longer than that and it is a
//! pause, which speech and music both have on purpose.
//!
//! **Stepped gain** is found as a series of *steps*: at every block boundary,
//! the level just after it against the level just before it, each side a
//! Hann-weighted window so a tone's own ripple does not read as a step. A
//! staircase is then a run of steps that all go the same way, land at a
//! regular period, and have flat level between them — which is what tells it
//! from a smooth fade (whose step is the same everywhere), from a low tone's
//! ripple (which goes up and down in turn), and from noise (which is not
//! regular). The window is chosen per period: a window wider than half the
//! period would straddle two steps, and a narrower one is noisier, so each
//! [`SCALES`] entry looks only for the periods it fits.

use super::Found;

/// The length of one block of energy, in seconds.
const BLOCK: f64 = 0.0005;

/// How far under the sound either side a dropout falls, in decibels.
const DEPTH_DB: f64 = 40.0;
/// How loud the sound either side of a dropout must be, as a mean square:
/// −40 dBFS. A gap in a whisper of room tone is not a dropout anybody hears.
const SOUNDING: f64 = 1e-4;
/// How much sound either side of a gap is read, in seconds.
const AROUND: f64 = 0.02;
/// The shortest and the longest a dropout is, in seconds.
const SHORTEST_DROPOUT: f64 = 0.001;
const LONGEST_DROPOUT: f64 = 0.04;

/// Each side's window, in blocks, for stepped gain. A scale looks for periods
/// from twice its window to four times it, so together they cover 4 ms (a
/// 250 Hz control rate) to 64 ms (a game running at 16 frames a second).
const SCALES: [usize; 4] = [4, 8, 16, 32];
/// The smallest step worth reporting, in decibels.
const STEP_DB: f64 = 0.25;
/// How many steps in a row make a staircase.
const STEPS: usize = 5;
/// How much quieter than its steps a staircase's flat between them must be.
const FLAT: f64 = 0.4;
/// Under this mean square a window is silence and has no level to step.
const SILENT: f64 = 1e-7;

/// One channel's energy, a block at a time.
#[derive(Debug, Clone)]
pub(super) struct Envelope {
    block: usize,
    /// The mean square of each whole block so far.
    energy: Vec<f64>,
    sum: f64,
    filled: usize,
}

impl Envelope {
    pub(super) fn new(rate: u32) -> Self {
        Self {
            block: ((BLOCK * f64::from(rate)).round() as usize).max(1),
            energy: Vec::new(),
            sum: 0.0,
            filled: 0,
        }
    }

    pub(super) fn feed(&mut self, sample: f32) {
        self.sum += f64::from(sample) * f64::from(sample);
        self.filled += 1;
        if self.filled == self.block {
            self.energy.push(self.sum / self.block as f64);
            self.sum = 0.0;
            self.filled = 0;
        }
    }

    /// The frame a block boundary falls on.
    fn frame(&self, block: usize) -> u64 {
        (block * self.block) as u64
    }

    fn blocks(&self, seconds: f64) -> usize {
        ((seconds / BLOCK).round() as usize).max(1)
    }

    /// The mean energy of `blocks`.
    fn mean(&self, blocks: std::ops::Range<usize>) -> f64 {
        let len = blocks.len().max(1) as f64;
        self.energy[blocks].iter().sum::<f64>() / len
    }

    pub(super) fn dropouts(&self) -> Vec<Found> {
        let (around, shortest, longest) = (
            self.blocks(AROUND),
            self.blocks(SHORTEST_DROPOUT),
            self.blocks(LONGEST_DROPOUT),
        );
        let deep = 10f64.powf(-DEPTH_DB / 10.0);
        let mut found = Vec::new();
        let mut at = around;
        while at + around < self.energy.len() {
            let before = self.mean(at - around..at);
            if before < SOUNDING || self.energy[at] >= before * deep {
                at += 1;
                continue;
            }
            let end = (at..self.energy.len())
                .find(|&k| self.energy[k] >= before * deep)
                .unwrap_or(self.energy.len());
            let length = end - at;
            if (shortest..=longest).contains(&length) && end + around <= self.energy.len() {
                let after = self.mean(end..end + around);
                let gap = self.mean(at..end).max(f64::MIN_POSITIVE);
                let size_db = 10.0 * (before.min(after) / gap).log10();
                if after >= SOUNDING && size_db >= DEPTH_DB {
                    found.push(Found {
                        at: self.frame(at),
                        from: self.frame(at),
                        to: self.frame(end),
                        size_db: size_db.min(120.0),
                        period: None,
                    });
                }
            }
            at = end.max(at + 1);
        }
        found
    }

    pub(super) fn zipper(&self) -> Vec<Found> {
        SCALES
            .iter()
            .flat_map(|&width| self.staircases(width))
            .collect()
    }

    /// The staircases whose period suits a window of `width` blocks a side.
    fn staircases(&self, width: usize) -> Vec<Found> {
        let steps = self.steps(width);
        let mut found = Vec::new();
        let mut run: Vec<usize> = Vec::new();
        for at in (width..steps.len()).filter(|&at| is_step(&steps, at, width)) {
            let regular = match run.as_slice() {
                [] => false,
                [only] => (2 * width..4 * width).contains(&(at - only)),
                [first, .., last] => {
                    let period = (last - first) as f64 / (run.len() - 1) as f64;
                    ((at - last) as f64 - period).abs() <= (0.1 * period).max(1.5)
                }
            };
            let same_way = run
                .last()
                .is_some_and(|&last| steps[last].signum() == steps[at].signum());
            if !(regular && same_way) {
                found.extend(self.staircase(&steps, &run));
                run.clear();
            }
            run.push(at);
        }
        found.extend(self.staircase(&steps, &run));
        found
    }

    /// A run of steps, as a staircase, if it is long enough and flat between.
    fn staircase(&self, steps: &[f64], run: &[usize]) -> Option<Found> {
        let (&first, &last) = (run.first()?, run.last()?);
        if run.len() < STEPS {
            return None;
        }
        let mut sizes: Vec<f64> = run.iter().map(|&at| steps[at].abs()).collect();
        let between: f64 = run
            .windows(2)
            .map(|pair| steps[(pair[0] + pair[1]) / 2].abs())
            .sum::<f64>()
            / (run.len() - 1) as f64;
        let mean = sizes.iter().sum::<f64>() / sizes.len() as f64;
        if between > FLAT * mean {
            return None;
        }
        sizes.sort_by(f64::total_cmp);
        Some(Found {
            at: self.frame(first),
            from: self.frame(first),
            to: self.frame(last),
            size_db: sizes[sizes.len() / 2],
            period: Some((last - first) as f64 * self.block as f64 / (run.len() - 1) as f64),
        })
    }

    /// The step at each block boundary, in decibels: after over before.
    /// Zero where either side is silent, or too near an end to have a side.
    fn steps(&self, width: usize) -> Vec<f64> {
        let weights: Vec<f64> = (0..width)
            .map(|j| 0.5 - 0.5 * (std::f64::consts::TAU * (j as f64 + 0.5) / width as f64).cos())
            .collect();
        let total: f64 = weights.iter().sum();
        let mut steps = vec![0.0; self.energy.len()];
        for (at, step) in steps.iter_mut().enumerate().skip(width) {
            if at + width > self.energy.len() {
                break;
            }
            let side = |from: usize| {
                weights
                    .iter()
                    .zip(&self.energy[from..from + width])
                    .map(|(w, e)| w * e)
                    .sum::<f64>()
                    / total
            };
            let (before, after) = (side(at - width), side(at));
            if before >= SILENT && after >= SILENT {
                *step = 10.0 * (after / before).log10();
            }
        }
        steps
    }
}

/// Whether the boundary `at` is a step: large enough, and the largest within
/// a window either side (the earlier of two equal ones).
fn is_step(steps: &[f64], at: usize, width: usize) -> bool {
    let here = steps[at].abs();
    here >= STEP_DB
        && (at.saturating_sub(width)..at).all(|k| steps[k].abs() < here)
        && (at + 1..(at + width + 1).min(steps.len())).all(|k| steps[k].abs() <= here)
}
