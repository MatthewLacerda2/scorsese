//! Every integration test of `core`, in one binary (#1128).
//!
//! Each module is what used to be a test binary of its own; one binary
//! links once instead of once per file.

mod animation;
mod annotating;
mod arrows;
mod briefing;
mod common;
mod cropping;
mod ducking;
mod examples;
mod fills;
mod fitting;
mod format;
mod frozen;
mod grid;
mod grouping;
mod keying;
mod levelling;
mod lighting;
mod matting;
mod migration_table;
mod optics;
mod pacing;
mod pivoting;
mod pool;
mod properties;
mod round_trip;
mod speeds;
mod stamping;
mod styling;
mod taping;
mod validation;
mod writing;
