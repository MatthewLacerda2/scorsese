//! Synthesis at the library level: the things a CLI test cannot reach — a
//! recipe that tries to leave the project, and a song whose instrument lives
//! in its own file.

#[path = "../common/mod.rs"]
mod common;

mod instruments;
mod safety;
