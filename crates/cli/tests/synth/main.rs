//! `scorsese synth`, driven end to end against real project directories.
//!
//! Nothing here is mocked and nothing needs a network, because synthesis is
//! arithmetic — these tests run the real thing and listen to the result.

#[path = "../common/mod.rs"]
mod common;

mod authoring;
mod baking;
