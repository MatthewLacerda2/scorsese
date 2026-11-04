//! A click: one sample-to-sample jump far larger than every other jump around
//! it.
//!
//! **Every other jump, not the typical one.** A sawtooth jumps by its whole
//! height once a period, a drum's attack is a run of large jumps, and noise is
//! nothing but jumps; none of them clicks. What clicks is a jump that stands
//! alone — a sample-accurate cut in a tone, a buffer spliced at the wrong
//! place — so a jump is measured against the *largest* jump in the
//! [`REACH`] either side of it, which a regular waveform always has a twin of
//! and a click never does. Two samples either side are left out, so an
//! impulse (up then straight back down) is not its own twin.

use std::collections::VecDeque;

use super::Found;

/// How far either side a jump is compared, in seconds. Twenty-five
/// milliseconds holds a full period of anything down to 40 Hz, so a bass
/// sawtooth always finds its next jump inside it.
const REACH: f64 = 0.025;

/// How many samples either side of a jump are not its neighbours.
const OWN: usize = 2;

/// How many times larger than every neighbour a jump must be. Twelve decibels.
const RATIO: f32 = 4.0;

/// The smallest jump worth calling a click: −40 dBFS. Under it a jump is too
/// small to hear over anything.
const FLOOR: f32 = 0.01;

/// The largest neighbour a jump is ever compared against, from below, so a
/// click out of digital silence reads as a large finite number.
const QUIET: f32 = 1e-4;

/// Clicks closer together than this, in seconds, are one click: an impulse
/// is a jump up and a jump straight back.
const ONE_CLICK: f64 = 0.005;

/// The click search for one channel.
#[derive(Debug, Clone)]
pub(super) struct Clicks {
    reach: usize,
    one_click: u64,
    previous: Option<f32>,
    /// The size of the most recent jumps, oldest first: never more than
    /// twice [`REACH`] and the jump being judged.
    recent: VecDeque<f32>,
    /// Which sample [`Clicks::recent`]'s first jump lands on.
    first: u64,
    /// How many samples were fed.
    fed: u64,
    /// The click being gathered, which may yet be joined by a larger one.
    pending: Option<Found>,
    found: Vec<Found>,
}

impl Clicks {
    pub(super) fn new(rate: u32) -> Self {
        Self {
            reach: ((REACH * f64::from(rate)) as usize).max(OWN + 1),
            one_click: (ONE_CLICK * f64::from(rate)) as u64,
            previous: None,
            recent: VecDeque::new(),
            first: 1,
            fed: 0,
            pending: None,
            found: Vec::new(),
        }
    }

    pub(super) fn feed(&mut self, sample: f32) {
        self.fed += 1;
        let Some(previous) = self.previous.replace(sample) else {
            return;
        };
        self.recent.push_back((sample - previous).abs());
        if self.recent.len() > 2 * self.reach + 1 {
            self.recent.pop_front();
            self.first += 1;
        }
        // The jump `reach` back now has all of its right-hand neighbours.
        let newest = self.fed - 1;
        if newest > self.reach as u64 {
            self.judge(newest - self.reach as u64);
        }
    }

    /// Every click, the jumps still short of their right-hand neighbours
    /// judged as if the signal ended here.
    pub(super) fn finish(&self) -> Vec<Found> {
        let mut rest = self.clone();
        let newest = self.fed.saturating_sub(1);
        let from = (newest + 1).saturating_sub(self.reach as u64).max(1);
        for at in from..=newest {
            if self.fed >= 2 {
                rest.judge(at);
            }
        }
        rest.found.extend(rest.pending.take());
        rest.found
    }

    /// Whether the jump onto sample `at` is a click.
    fn judge(&mut self, at: u64) {
        let Some(here) = at.checked_sub(self.first).map(|pos| pos as usize) else {
            return;
        };
        let jump = self.recent[here];
        if jump < FLOOR {
            return;
        }
        let neighbours = (here.saturating_sub(self.reach)..here.saturating_sub(OWN))
            .chain(here + OWN + 1..(here + self.reach + 1).min(self.recent.len()));
        let mut loudest = QUIET;
        for pos in neighbours {
            loudest = loudest.max(self.recent[pos]);
            if jump < RATIO * loudest {
                return;
            }
        }
        let size_db = 20.0 * f64::from(jump / loudest).log10();
        self.found_at(at, size_db);
    }

    fn found_at(&mut self, at: u64, size_db: f64) {
        let one = Found {
            at,
            from: at.saturating_sub(self.one_click),
            to: at + self.one_click,
            size_db,
            period: None,
        };
        match &mut self.pending {
            Some(pending) if at - pending.at <= self.one_click => {
                if size_db > pending.size_db {
                    pending.at = at;
                    pending.size_db = size_db;
                }
                pending.to = one.to;
            }
            _ => self.found.extend(self.pending.replace(one)),
        }
    }
}
