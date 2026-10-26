//! Every integration test of `cli`, in one binary (#1128).
//!
//! Each module is what used to be a test binary of its own; one binary
//! links once instead of once per file.

mod check;
mod commands;
mod common;
mod describing;
mod ducking;
mod generating;
mod guiding;
mod help;
mod piping;
mod render;
mod sealed;
mod still;
mod synth;
mod voicing;
