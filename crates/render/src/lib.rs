//! # scorsese-render — ffmpeg orchestration
//!
//! Responsibility: everything that touches ffmpeg/ffprobe — probing imported
//! media, decoding sources to raw frames over pipes, piping composited raw
//! frames into an ffmpeg encode process, and render settings (aspect,
//! resolution, fps, bitrate — user-chosen per render).
//!
//! Every ffmpeg invocation in the entire workspace goes through this crate's
//! command builder. No ad-hoc `Command::new("ffmpeg")` anywhere else, ever.
//! In dev/CI ffmpeg is an external binary on PATH; in shipped builds it is a
//! bundled Tauri sidecar — this crate is the one place that indirection lives.
//!
//! Boundary: no compositing logic (that is `scorsese-compositor`'s job — this
//! crate moves bytes, it never draws), no provider calls, no GUI. Depends on
//! `scorsese-core` and `scorsese-compositor`.

/// Placeholder so `cargo test` exercises this crate from day one.
/// Replaced by real pipeline tests in the render-pipeline issue.
#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
