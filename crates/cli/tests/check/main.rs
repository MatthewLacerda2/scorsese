//! `scorsese check` — what it says about the media the timeline references.
//!
//! The real command over real project directories, asserting the thing a
//! caller actually branches on — whether it failed — and what it said while
//! doing it. What counts as a fault at all is `scorsese-render`'s `Checkup`
//! and is tested there, beside the assembly both this command and the MCP
//! server's `project_check` read their answer out of.

#[path = "../common/mod.rs"]
mod common;

mod exit_codes;
mod glyphs;
mod scripts;
mod verify;
