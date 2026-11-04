//! The test seam for [`Handles::bounds`](super::Handles).

use std::time::Duration;

use super::super::Toolbox;
use crate::tools::bound::Bounds;

impl Toolbox {
    /// The same tools, with at most `at_once` of a member's registry tool
    /// calls running at a time and each stopped past `deadline` — a test's,
    /// which cannot wait [`DEADLINE`](crate::tools::DEADLINE).
    pub fn bounded(mut self, at_once: usize, deadline: Duration) -> Self {
        self.handles.bounds = Bounds::new(at_once, deadline);
        self
    }
}
