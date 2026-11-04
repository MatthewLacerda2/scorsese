//! Dropouts and stepped gain, both read off a channel's energy every half
//! millisecond.
//!
//! **A dropout** is a run of blocks [`DEPTH_DB`] under the sound on both sides
//! of it, no longer than [`LONGEST_DROPOUT`]: longer than that and it is a
//! pause, which speech and music both have on purpose.
//!
//! **Stepped gain** is [`super::zipper`]'s, read off the energy of the
//! envelope, which is kept beside the waveform's.

use super::Found;
use super::quadrature::Quadrature;

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

/// One channel's energy, a block at a time.
#[derive(Debug, Clone)]
pub(super) struct Envelope {
    block: usize,
    quadrature: Quadrature,
    /// The mean square of each whole block so far.
    energy: Vec<f64>,
    sum: f64,
    /// The same, of the envelope rather than the waveform: what stepped gain
    /// is read from. A dropout is read from the waveform, because the
    /// envelope's all-pass chains ring on through a gap.
    envelope: Vec<f64>,
    envelope_sum: f64,
    filled: usize,
}

impl Envelope {
    pub(super) fn new(rate: u32) -> Self {
        Self {
            block: ((BLOCK * f64::from(rate)).round() as usize).max(1),
            quadrature: Quadrature::default(),
            energy: Vec::new(),
            sum: 0.0,
            envelope: Vec::new(),
            envelope_sum: 0.0,
            filled: 0,
        }
    }

    pub(super) fn feed(&mut self, sample: f32) {
        self.sum += f64::from(sample) * f64::from(sample);
        self.envelope_sum += self.quadrature.energy(sample);
        self.filled += 1;
        if self.filled == self.block {
            self.energy.push(self.sum / self.block as f64);
            self.envelope.push(self.envelope_sum / self.block as f64);
            self.sum = 0.0;
            self.envelope_sum = 0.0;
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
        super::zipper::staircases(&self.envelope, self.block)
    }
}
