//! # scorsese-core — the project model
//!
//! Responsibility: the timeline model, the assets table, clips, keyframe
//! tracks, the serde `project.json` format, and validation (dangling asset
//! references, overlap constraints, path hygiene).
//!
//! A project is a directory (`*.scor/`): `project.json` plus `assets/`,
//! `generated/`, and `cache/`. Every path stored in `project.json` is
//! relative to the project root — never absolute — so a project survives
//! `scp -r` between machines. Assets are entities (id, kind, path, sha256,
//! probed metadata); clips reference assets by id, never by path.
//!
//! Keyframe tracks are generic: `(property_path, [(t, value, easing)])` over
//! any numeric property. This crate defines property *types*, never property
//! *values*.
//!
//! Boundary: this crate must never touch a display, a GPU, a network, or
//! spawn a process. No GUI, no ffmpeg, no provider calls — it depends on
//! nothing but serde-level data handling. Everything else depends on it.

/// Placeholder so `cargo test` exercises this crate from day one.
/// Replaced by real model tests in the project.json schema v1 issue.
#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
