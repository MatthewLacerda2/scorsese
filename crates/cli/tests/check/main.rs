//! `scorsese check` — what it says about the media the timeline references.
//!
//! Two layers, because they fail for different reasons. `severity` pins the
//! health→severity table with hand-built pool rows and no filesystem at all;
//! `exit_codes` drives the real command over real project directories, and
//! only asserts the thing a caller actually branches on — whether it failed.

#[path = "../common/mod.rs"]
mod common;

mod exit_codes;
mod severity;
mod verify;
