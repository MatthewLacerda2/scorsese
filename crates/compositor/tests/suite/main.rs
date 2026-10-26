//! Every integration test of `compositor`, in one binary (#1128).
//!
//! Each module is what used to be a test binary of its own; one binary
//! links once instead of once per file.

mod aberrating;
mod blending;
mod blurring;
mod common;
mod coordinates;
mod dissolving;
mod flips;
mod grading;
mod icons;
mod keying;
mod layers;
mod lighting;
mod matting;
mod offscreen;
mod pivots;
mod properties;
mod registry;
mod resampling;
mod resting;
mod ruler;
mod shapes;
mod sheet;
mod taping;
mod text;
mod transforms;
mod waveform;
