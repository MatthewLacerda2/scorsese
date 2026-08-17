//! What is wrong with a project, which is one answer for every caller.
//!
//! Two layers, because they fail for different reasons. `severity` pins the
//! health→severity table with hand-built pool rows and no filesystem at all;
//! `assembly` puts a whole project in front of a checkup and asserts that each
//! of the four sources it draws on actually reaches the report.
//!
//! No ffmpeg anywhere in here, and that is the feature as much as the test: a
//! checkup is meant to be cheap enough to run after every edit.

#[path = "../common/mod.rs"]
mod common;

mod assembly;
mod overlaps;
mod severity;
