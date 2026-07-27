//! The size gate: source files ≤ 300 lines, test files ≤ 150.
//!
//! CLAUDE.md lists this among the hard gates, next to `clippy -D warnings` and
//! the golden renders. Small files are what let an agent hold a module in
//! context and change it confidently, which is what makes unattended work
//! possible at all — so the limits are checked rather than remembered.
//!
//! Two halves, deliberately separate: [`classify`] decides *which* cap a path
//! falls under and is the part with its own tests, since a classifier that
//! quietly puts a test file in the source bucket is a gate that has stopped
//! gating without saying so. [`scan`] walks a directory and counts lines.
//!
//! There is no baseline file and nothing is grandfathered. If a file is over,
//! it gets split.

pub mod classify;
pub mod scan;

pub use classify::{Kind, SOURCE_LIMIT, TEST_LIMIT, classify};
pub use scan::{Report, Violation, check_dir};
