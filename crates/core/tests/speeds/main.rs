//! How fast a clip runs against the timeline it sits on.
//!
//! Split along the line the field is built on: `speed` is a rate the document
//! records, and rescaling `duration` to match is an operation something
//! performs. Neither half is allowed to quietly become the other.

#[path = "../common/mod.rs"]
mod common;

mod document;
mod operations;
