//! Delivering a soundtrack at a loudness target (#968).
//!
//! A feed normalises what it plays, so a quiet ad plays quieter than everything
//! around it, and *make it as loud as the others* is the request. Without this
//! a render can only turn a mix **down** ([`super::headroom`]); bringing one up
//! by hand stops at the first peak to reach the ceiling, because nothing holds
//! the peaks. So, when a render asks for a target:
//!
//! 1. The finished mix is measured in LUFS ([`Integrated`], BS.1770's gated
//!    loudness — the unit platforms publish their figures in).
//! 2. It is raised (or lowered) by the difference, and every peak that gain
//!    pushes over the ceiling is held by a true-peak [`limiter::Limiter`].
//!    Limiting takes some loudness back, so the result is measured and the
//!    gain corrected until it lands within [`SETTLED`] of the target.
//! 3. The headroom rehearsal runs as it always does. When a lossy codec still
//!    needs room, the trim it makes would land the file that far under the
//!    target, so instead the mix is raised again from the untouched original
//!    with the limiter's ceiling lowered by exactly that room — the target is
//!    kept and the codec gets its headroom out of the peaks.
//!
//! Off unless asked for: without a target this module calls the rehearsal and
//! nothing else, so every existing render is what it was, bit for bit.

pub(crate) mod limiter;
pub(crate) mod pass;
pub(crate) mod target;

use std::fmt;
use std::path::{Path, PathBuf};

use scorsese_zimmer::level::{Integrated, Loudness, Meter};

use super::headroom::{self, DELIVERY_CEILING_DBTP, Trim};
use super::mix::CHANNELS;
use super::read::{self, ANALYSIS_RATE};
use crate::error::RenderError;
use crate::settings::RenderSettings;
use crate::tools::Tools;

use target::LoudnessTarget;

/// How close to the target is close enough, in LU: the tolerance EBU Tech
/// 3341 allows a meter, so nothing tighter could be told apart.
const SETTLED: f64 = 0.1;

/// How far under its target a delivered file may land before the report says
/// it fell short, in LU: past what a lossy codec and a meter disagree by.
const SHORT_OF_TARGET: f64 = 0.5;

/// How many times the gain is corrected for what the limiter took back.
///
/// The limiter's cost grows more slowly than the gain that causes it, so each
/// correction lands closer; the first nearly always settles it. Past this the
/// mix is delivered as close as it got, and the report says how close.
const ATTEMPTS: usize = 4;

/// The most gain added beyond the plain difference to make up for what the
/// limiter takes back, in dB.
///
/// A mix whose loudness lives in its peaks — a drum hit, a burst of distortion
/// — can only be made louder by limiting those peaks harder, and every decibel
/// of gain then buys a fraction of one of loudness while flattening what made
/// it loud. Six decibels of make-up is already heavy limiting; past it the
/// render stops short, and the report says by how much, because a target the
/// mix cannot reach without being crushed is the author's call, not ours.
const MAKEUP_DB: f64 = 6.0;

/// How many times the mix is raised against a lossy codec's overshoot. The
/// overshoot is close to proportional to the peaks, so one lowering of the
/// ceiling nearly fits it — but a heavily limited mix overshoots a little more
/// each time the ceiling comes down, and on square bursts through AAC the
/// second lowering was needed (#968). Whatever is left after the last round
/// is trimmed, as any render's would be, and the report says how short that
/// left it.
const ROUNDS: usize = 3;

/// What a render did to bring its soundtrack to a loudness target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lift {
    /// The loudness asked for.
    pub target: LoudnessTarget,
    /// The mix as its author balanced it, in LUFS. `None` when there was no
    /// loudness to measure — silence, or under the 400 ms one block needs —
    /// and so nothing was done.
    pub mixed_lufs: Option<f64>,
    /// How far the whole mix was moved, in dB: positive raised, negative
    /// lowered.
    pub gain_db: f64,
    /// How far the limiter's deepest reduction went, in dB. `0.0` when no peak
    /// needed holding.
    pub limited_db: f64,
    /// The true peak the limiter held the mix under, in dBTP: the delivery
    /// ceiling, lowered by whatever room a lossy codec needed on top.
    pub ceiling_dbtp: f64,
    /// The delivered file's loudness, decoded as a player would, in LUFS.
    pub delivered_lufs: Option<f64>,
}

impl fmt::Display for Lift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(mixed) = self.mixed_lufs else {
            return write!(
                f,
                "no loudness to bring to {}: the soundtrack is silent, or shorter than \
                 the 0.4 s it takes to measure",
                self.target
            );
        };
        let moved = if self.gain_db >= 0.0 {
            "raised"
        } else {
            "lowered"
        };
        write!(f, "{mixed:.1} LUFS as mixed")?;
        if let Some(delivered) = self.delivered_lufs {
            write!(f, ", {delivered:.1} LUFS delivered")?;
        }
        write!(
            f,
            " (target {}): {moved} {:.1} dB",
            self.target,
            self.gain_db.abs()
        )?;
        if let Some(short) = self
            .delivered_lufs
            .map(|delivered| self.target.lufs() - delivered)
            .filter(|short| *short > SHORT_OF_TARGET)
        {
            write!(
                f,
                " — {short:.1} LU short: its loudness is in its peaks, and they would \
                 not go higher without being crushed"
            )?;
        }
        if self.limited_db >= 0.05 {
            write!(
                f,
                ", the limiter taking up to {:.1} dB off peaks to hold them under {:.1} dBTP",
                self.limited_db, self.ceiling_dbtp
            )?;
        }
        Ok(())
    }
}

/// Brings the mix at `mix` to the settings' loudness target, when there is
/// one, then fits it under a lossy codec's overshoot. What was done to it, in
/// that order: the trim the rehearsal made, and the lift.
pub(crate) fn deliver(
    tools: &Tools,
    settings: &RenderSettings,
    mix: &Path,
    out: &Path,
) -> Result<(Option<Trim>, Option<Lift>), RenderError> {
    let Some(target) = settings.loudness else {
        return Ok((headroom::fit(tools, settings, mix, out)?, None));
    };
    let rate = settings.sample_rate.hz();
    let scratch = |path: &Path| {
        let path = path.to_owned();
        move |source| RenderError::Scratch { path, source }
    };
    let mut lift = Lift {
        target,
        mixed_lufs: pass::loudness(mix, rate).map_err(scratch(mix))?,
        gain_db: 0.0,
        limited_db: 0.0,
        ceiling_dbtp: DELIVERY_CEILING_DBTP,
        delivered_lufs: None,
    };
    let Some(mixed) = lift.mixed_lufs else {
        return Ok((headroom::fit(tools, settings, mix, out)?, Some(lift)));
    };
    // The untouched mix, kept so that every attempt starts from what the
    // author balanced rather than from the last attempt's limiting.
    let original = Original(super::scratch_beside(out, "scorsese-unraised.pcm"));
    std::fs::rename(mix, &original.0).map_err(scratch(mix))?;
    let mut trim = None;
    for round in 0..ROUNDS {
        (lift.gain_db, lift.limited_db) =
            reach(&original.0, mix, rate, target, mixed, lift.ceiling_dbtp)?;
        trim = headroom::fit(tools, settings, mix, out)?;
        match trim {
            Some(room) if round + 1 < ROUNDS => lift.ceiling_dbtp -= room.trimmed_db,
            _ => break,
        }
    }
    Ok((trim, Some(lift)))
}

/// Writes `original` to `mix` raised toward `target` and limited under
/// `ceiling_dbtp`, correcting the gain for what the limiter takes back. Hands
/// back the gain the mix was finally written at and the limiter's deepest
/// reduction.
fn reach(
    original: &Path,
    mix: &Path,
    rate: u32,
    target: LoudnessTarget,
    mixed: f64,
    ceiling_dbtp: f64,
) -> Result<(f64, f64), RenderError> {
    let planned = target.lufs() - mixed;
    let mut gain = planned;
    let mut limited = 0.0;
    for attempt in 1..=ATTEMPTS {
        let raised = pass::raise(original, mix, rate, gain, ceiling_dbtp).map_err(|source| {
            RenderError::Scratch {
                path: mix.to_owned(),
                source,
            }
        })?;
        limited = raised.limited_db;
        let Some(reached) = raised.lufs else { break };
        let short = target.lufs() - reached;
        // The last attempt stays as written, so the gain handed back is the
        // one the mix is actually at.
        let next = (gain + short).min(planned + MAKEUP_DB);
        if short.abs() <= SETTLED || attempt == ATTEMPTS || next - gain < SETTLED {
            break;
        }
        gain = next;
    }
    Ok((gain, limited))
}

/// How loud the soundtrack of a finished file is, as [`headroom::measure`]
/// says, and its integrated loudness — one decode for both.
pub(crate) fn measure(tools: &Tools, file: &Path) -> Result<(Loudness, Option<f64>), RenderError> {
    let mut meter = Meter::new(CHANNELS);
    let mut integrated = Integrated::new(CHANNELS, ANALYSIS_RATE);
    read::decode(tools, file, CHANNELS, |samples| {
        meter.feed(samples);
        integrated.feed(samples);
    })?;
    Ok((meter.finish(), integrated.finish()))
}

/// The mix as mixed, removed when dropped — including when a render fails part
/// way through raising it.
struct Original(PathBuf);

impl Drop for Original {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
