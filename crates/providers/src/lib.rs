//! # scorsese-providers — generation backends
//!
//! Responsibility: clients for generative providers (Veo for video,
//! ElevenLabs for TTS audio), the prompt-hash cache under `generated/`
//! (content-addressed — an unchanged prompt is never regenerated), and the
//! generation state machine: `sketch → queued → generated → stale`.
//!
//! Credentials come from `.env` (gitignored; `.env.example` documents the
//! keys). Real provider calls are NEVER made in tests — every provider is
//! mocked behind a trait.
//!
//! Boundary: no rendering, no compositing, no GUI. This crate turns prompts
//! into media files on disk and updates asset state; it depends on
//! `scorsese-core` only.

/// Placeholder so `cargo test` exercises this crate from day one.
/// Replaced by mocked provider tests in the provider issues.
#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
