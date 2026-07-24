//! # scorsese-compositor — frame rendering
//!
//! Responsibility: producing each output frame from decoded source frames —
//! transforms (position/scale), alpha blending, opacity, text rendering,
//! image overlay, and slug cards for sketch/stale prompt clips. CPU-first
//! via tiny-skia; a wgpu backend arrives later behind the same trait.
//!
//! Compositing is ours; ffmpeg only decodes and encodes (Path B). This crate
//! consumes raw frames and keyframe-evaluated property values, and emits raw
//! frames. It knows property *types* (a numeric property animated over time),
//! never which property it animates.
//!
//! Boundary: no ffmpeg invocation, no encoding, no file I/O on media, no
//! provider calls, no GUI event loop. It depends on `scorsese-core` for the
//! model and nothing above it.

/// Placeholder so `cargo test` exercises this crate from day one.
/// Replaced by real compositing tests in the Compositor v1 issue.
#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
