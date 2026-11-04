//! The handles every tool call may share, and the limits it runs within: one
//! struct, and each handle's test seam in its own file, so a new shared
//! handle adds a file here rather than a line to a list every feature edits.
//!
//! - [`stock`]: the library stock searches and imports answer from, and each
//!   member's share of it.
//! - [`voices`]: where a voice's origin is asked.
//! - [`bounds`]: how many of a member's calls run at once, and for how long.

mod bounds;
mod stock;
mod voices;

pub use voices::Voices;

use super::bound::{AT_ONCE, Bounds, DEADLINE};
use super::share::Shares;
use scorsese_mcp::Stock;

/// What [`super::Toolbox`] shares between users' calls. Cheap to clone.
#[derive(Clone)]
pub(super) struct Handles {
    /// The one library every stock search and import answers from, when not
    /// each medium's own.
    pub(super) stock: Option<Stock>,
    /// How much of each stock library each member has used this minute.
    pub(super) shares: Shares,
    /// Where a voice's origin is asked, when not ElevenLabs keyed from the one
    /// resolver — a test's.
    pub(super) voices: Option<Voices>,
    /// How many of a member's registry tool calls run at once, and for how
    /// long.
    pub(super) bounds: Bounds,
}

impl Default for Handles {
    fn default() -> Self {
        Self {
            stock: None,
            shares: Shares::default(),
            voices: None,
            bounds: Bounds::new(AT_ONCE, DEADLINE),
        }
    }
}
