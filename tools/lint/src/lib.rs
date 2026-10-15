//! The tree's lint: the size gate, and the conventions a machine can hold.
//!
//! **The size gate**: source files ≤ 300 lines of code, test files ≤ 150.
//! CLAUDE.md lists this among the hard gates, next to `clippy -D warnings` and
//! the golden renders. Small files are what let an agent hold a module in
//! context and change it confidently, which is what makes unattended work
//! possible at all — so the limits are checked rather than remembered.
//!
//! *Lines of code* means blank and comment lines do not count; [`scan::code_lines`]
//! says why at length. The short of it is that this repo documents deliberately
//! and at length, and a cap that counted prose was splitting files whose code
//! was never the problem.
//!
//! Its halves are deliberately separate: [`classify`] decides *which* cap a
//! path falls under and is the part with its own tests, since a classifier that
//! quietly puts a test file in the source bucket is a gate that has stopped
//! gating without saying so. [`tree`] walks the repository once for every
//! check; [`scan`] counts lines; [`python`] counts them for Python, whose
//! comments and docstrings are shaped differently and are just as free.
//!
//! There is no baseline file and nothing is grandfathered. If a file is over,
//! it gets split.
//!
//! **The conventions** ([`rules`], #1229): written rules a check can hold, so
//! they are held by one — group by subfolder, nothing temporary, tests never
//! skip, crate docs name their modules, no code `.gitignore` swallows. Each takes exceptions inline, with a
//! written reason.

pub mod classify;
pub mod python;
pub mod rules;
pub mod scan;
pub mod tree;

pub use classify::{Kind, SOURCE_LIMIT, TEST_LIMIT, classify};
pub use rules::{Finding, check_tree};
pub use scan::{Report, Violation, check_dir};
