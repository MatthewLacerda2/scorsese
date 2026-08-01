//! Which ffmpeg produced a set of reference frames, recorded beside them.
//!
//! The gate's premise is that encoders are not reproducible and frames are, so
//! frames are what gets compared. That reasoning is about the encoder at the
//! *end* of a render. ffmpeg is also the **decoder** at the start of one: every
//! pixel the compositor works on arrived from it, and two builds far enough
//! apart need not hand over the same ones. The tolerances in
//! [`Tolerance`](crate::Tolerance) were sized for encoder noise and say so; they
//! were never sized to absorb a different decoder.
//!
//! So a reference records the ffmpeg it was blessed under, and a failure can
//! say "the pixels moved **and** you are on a different decoder" rather than
//! presenting as a plain regression. That is the difference between a mystery
//! and a decision — and it matters most in the one situation where re-blessing
//! would be most tempting and most wrong.
//!
//! It is **never a failure on its own**, and never a warning on a passing run.
//! Failing would make the suite unrunnable for anyone whose distribution ships
//! a different ffmpeg, which is a gate going red for a reason its author cannot
//! see in their own diff. Warning on every green run would train everyone to
//! scroll past it. It speaks where it is actionable: inside a report of frames
//! that already disagree.

use std::fmt;
use std::path::{Path, PathBuf};

/// The file, inside a fixture's `expected/` directory, naming an ffmpeg that
/// produces exactly these reference frames. One line, so a diff of it reads as
/// a sentence.
///
/// "Produces exactly these" rather than "blessed these", because that is the
/// claim worth making and the one that can be checked. Blessing satisfies it by
/// construction — the frames came out of that ffmpeg — and a record written any
/// other way has to earn it by measurement.
pub const RECORD_FILE: &str = "decoder.txt";

fn record_in(expected: &Path) -> PathBuf {
    expected.join(RECORD_FILE)
}

/// What decoded this run's frames, and what decoded the references it is being
/// held to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoder {
    pub(crate) running: String,
    /// `None` when a fixture carries no record — reported as unknown rather
    /// than assumed to match, because a guess here is worth less than nothing.
    pub(crate) recorded: Option<String>,
}

impl Decoder {
    /// An unreadable record is the same answer as a missing one: this is
    /// provenance, and no part of the gate should fail over it.
    pub(crate) fn read(expected: &Path, running: String) -> Self {
        let recorded = std::fs::read_to_string(record_in(expected))
            .ok()
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());
        Self { running, recorded }
    }

    /// Records the running ffmpeg as what blessed these references. Called
    /// when they are rewritten, so the record cannot go stale behind them.
    pub(crate) fn write(&self, expected: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(expected)?;
        std::fs::write(record_in(expected), format!("{}\n", self.running))
    }
}

impl fmt::Display for Decoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "  decoded by ffmpeg {}", self.running)?;
        match &self.recorded {
            Some(recorded) if recorded == &self.running => {
                write!(f, ", which is what these references were recorded under")
            }
            Some(recorded) => write!(
                f,
                ", but these references were recorded under ffmpeg {recorded}.\n  \
                 One thing to rule out: the decoder is upstream of this comparison, and \
                 the\n  tolerances above were sized for encoder noise rather than for a \
                 different one.\n  It is not a verdict — check it, then judge the frames. \
                 Re-blessing to make it\n  go away would bake this machine's decoder into \
                 the reference."
            ),
            None => write!(
                f,
                ", and these references carry no record of what produced them,\n  \
                 so a decoder difference can be neither ruled in nor out here."
            ),
        }
    }
}
