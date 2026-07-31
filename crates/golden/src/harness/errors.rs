//! Why a golden run did not come out clean.
//!
//! Split out from [`super`] to keep that file about running a fixture. One
//! variant is about pixels — [`GoldenError::Mismatch`] — and every other one is
//! the harness saying it could not get as far as comparing. Keeping that
//! distinction legible is why they are all here and all documented.

use std::path::PathBuf;

use scorsese_render::frames::FrameError;

use crate::compare::SizeMismatch;
use crate::fixture::FixtureError;

use super::{Mismatches, SetupError, UPDATE_VARIABLE};

/// Why a golden run did not come out clean.
#[derive(Debug, thiserror::Error)]
pub enum GoldenError {
    /// The one failure that is about pixels. Everything else here is the
    /// harness saying it could not get as far as comparing.
    #[error("golden fixture `{fixture}` does not match its references\n{mismatches}")]
    Mismatch {
        /// The fixture that regressed.
        fixture: String,
        /// Every frame that missed, and where to look at each.
        ///
        /// Boxed because it is by far the largest thing in this enum — a
        /// tolerance, the render path, a list of frames and the decoder
        /// record — and every other variant would otherwise be padded out to
        /// its size on every `Result` the harness returns.
        mismatches: Box<Mismatches>,
    },

    /// A frame nominated for comparison has no committed reference. Fails
    /// rather than being created, so a fixture never passes by asserting
    /// nothing.
    #[error(
        "golden fixture `{fixture}` has no reference for frame {frame} at {} — \
         create it with {UPDATE_VARIABLE}=1 cargo test -p scorsese-golden, \
         and look at it before committing it",
        path.display()
    )]
    NoReference {
        /// The fixture missing a reference.
        fixture: String,
        /// The frame it nominates but cannot be held to.
        frame: u64,
        /// Where the reference is expected to sit.
        path: PathBuf,
    },

    /// The fixture asks about a frame the render never produced — a timeline
    /// that got shorter, or a `range` narrowed without updating `frames`.
    #[error(
        "golden fixture `{fixture}` compares frame {frame}, but its render is only \
         {frames} frames long"
    )]
    FrameBeyondRender {
        /// The fixture that over-reaches.
        fixture: String,
        /// The frame it asks for.
        frame: u64,
        /// How many the render actually wrote.
        frames: u64,
    },

    /// Blessing rewrote the references but could not record what decoded them,
    /// which would leave the two disagreeing. Fails rather than being skipped:
    /// a record that silently stops being written is worse than none, because
    /// a stale one reads as current.
    #[error("golden fixture `{fixture}`: recording the decoder beside its references: {source}")]
    Record {
        /// The fixture whose record could not be written.
        fixture: String,
        /// What the filesystem said.
        #[source]
        source: std::io::Error,
    },

    /// The fixture disagrees with itself, before anything was rendered.
    #[error("the fixture is broken: {0}")]
    Fixture(#[from] FixtureError),

    /// Materialising the scratch project or conjuring its media failed.
    #[error("setting the fixture up: {0}")]
    Setup(#[from] SetupError),

    /// No ffmpeg on PATH — the harness needs one to render or to read a PNG.
    #[error(transparent)]
    Tools(#[from] scorsese_render::ToolsError),

    /// The render failed outright, which is a bug in the code under test and
    /// not something the fixture can be blamed for.
    #[error("rendering the fixture: {0}")]
    Render(#[from] scorsese_render::RenderError),

    /// A frame could not be pulled out of the render, or written back as a
    /// reference.
    #[error("reading a frame: {0}")]
    Frame(#[from] FrameError),

    /// The render and its reference are different sizes, so there is nothing
    /// to compare — re-bless deliberately if the resolution moved on purpose.
    #[error(transparent)]
    Size(#[from] SizeMismatch),
}
