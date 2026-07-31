//! Grouping a render's loudness: the mix, and each clip's contribution to it.
//!
//! The measurement itself is [`scorsese_soundgen::level`] — it is arithmetic
//! over samples and nothing to do with ffmpeg, and a bake needs exactly the
//! same numbers. What belongs here is the part that is about a *render*: that
//! there is one figure for the delivered soundtrack and one per audible clip.
//!
//! The mix number says something is wrong; the per-clip numbers say **which
//! clip**. In the noise-swell defect the mix was unremarkable and one track was
//! the problem, which is the case that argues for keeping both.

use std::collections::BTreeMap;

use scorsese_soundgen::level::{Loudness, Meter};

use super::mix::CHANNELS;

/// Every meter a mixdown keeps: one for the mix, one per audible clip.
#[derive(Debug, Clone)]
pub struct Levels {
    /// The finished soundtrack, as written.
    pub mix: Meter,
    /// Ordered by clip id so two runs of the same render report in the same
    /// order — a report that shuffles is a report nobody can diff.
    clips: BTreeMap<String, Meter>,
}

impl Default for Levels {
    fn default() -> Self {
        Self {
            mix: Meter::new(CHANNELS),
            clips: BTreeMap::new(),
        }
    }
}

impl Levels {
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
    /// The mix as delivered.
    pub mix: Loudness,
    /// Each audible clip's contribution, at the volume it was given, by clip
    /// id.
    pub clips: Vec<(String, Loudness)>,
}
