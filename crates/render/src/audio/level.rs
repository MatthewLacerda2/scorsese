//! Grouping a render's loudness: the mix, and each clip's contribution to it.
//!
//! The measurement itself is [`scorsese_zimmer::level`] — it is arithmetic
//! over samples and nothing to do with ffmpeg, and a bake needs exactly the
//! same numbers. What belongs here is the part that is about a *render*: that
//! there is one figure for the delivered soundtrack and one per audible clip.
//!
//! The mix number says something is wrong; the per-clip numbers say **which
//! clip**. In the noise-swell defect the mix was unremarkable and one track was
//! the problem, which is the case that argues for keeping both.
//!
//! **The mix is profiled and the clips are only metered**, and the asymmetry is
//! deliberate. A soundtrack that sags in its third minute is a finding, so the
//! delivered mix gets the sectioned table; a cut with forty clips in it would
//! get forty tables, which is a wall of text nobody reads to find the one row
//! that mattered. The mix's rows are what say *when*, and the clip list is what
//! says *which*.

use std::collections::BTreeMap;

use scorsese_zimmer::level::{Loudness, Meter, Profile, Profiler};

use super::mix::CHANNELS;

/// Every meter a mixdown keeps: one for the mix, one per audible clip.
#[derive(Debug, Clone)]
pub struct Levels {
    /// The finished soundtrack, as written.
    pub mix: Profiler,
    /// Ordered by clip id so two runs of the same render report in the same
    /// order — a report that shuffles is a report nobody can diff.
    clips: BTreeMap<String, Meter>,
}

impl Levels {
    /// Meters for a mix produced at `rate` samples a second.
    ///
    /// The rate is asked for rather than assumed because it is a render
    /// setting, and the sections of the mix are cut on a clock: a profiler that
    /// guessed 48 kHz would put every boundary in the wrong place for a render
    /// delivered at 44.1.
    pub fn new(rate: u32) -> Self {
        Self {
            mix: Profiler::new(CHANNELS, rate),
            clips: BTreeMap::new(),
        }
    }

    /// The meter for one clip, created the first time it is asked for.
    ///
    /// A clip appears in several segments when other tracks cut across it, and
    /// all of those are one clip's contribution — so the meter accumulates
    /// across them rather than being replaced.
    pub fn clip(&mut self, id: &str) -> &mut Meter {
        self.clips
            .entry(id.to_owned())
            .or_insert_with(|| Meter::new(CHANNELS))
    }

    /// What everything came out as.
    pub fn finish(&self) -> SoundLevels {
        SoundLevels {
            mix: self.mix.finish(),
            clips: self
                .clips
                .iter()
                .map(|(id, meter)| (id.clone(), meter.finish()))
                .collect(),
        }
    }
}

/// How loud a render's soundtrack came out, and each clip in it.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundLevels {
    /// The mix as delivered — whole, and section by section.
    pub mix: Profile,
    /// Each audible clip's contribution, at the volume it was given, by clip
    /// id.
    pub clips: Vec<(String, Loudness)>,
}
