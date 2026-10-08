//! Stock search and import, driven end to end against a scripted library.
//!
//! **Nothing here reaches a network or needs a key**: `Library` is a trait,
//! downloads included, so the 24-hour cache, the filtering, the paging and an
//! import into `assets/` all run against the fake in `fake.rs`.

#[path = "../common/mod.rs"]
mod common;

mod fake;
mod importing;
mod searching;
