//! An imported recording's speech, heard by a local Whisper (#1549).
//!
//! Word timings exist for a narration because its provider sends them. Real
//! footage — a sketch, an interview, a phone recording — has none, so this
//! asks a Whisper for them and writes them **beside the asset**, in the same
//! format ([`scorsese_core::words`]): captions, pages and `project_describe`
//! then read footage as they read a line.
//!
//! # What holds this together
//!
//! **Local, and only local.** Whisper large-v3-turbo, int8 on the CPU, in a
//! container of its own (`deploy/whisper/Dockerfile`) that the CLI, the MCP
//! server, the desktop app and the web app's jobs all reach over HTTP at an
//! [`address`]. No Whisper is compiled into scorsese, nothing is paid for,
//! and no vendor hears a user's recording.
//!
//! **The words are kept, never redone on their own.** A transcript the agent
//! corrected ([`Words::fix`](scorsese_core::words::Words::fix)) is work
//! nothing can rebuild, so [`transcribe`] reads an existing one back and asks
//! the server again only when told to ([`Asked::again`]).
//!
//! **This crate decodes nothing.** What is sent is a WAV the caller made —
//! `scorsese_render::audio::speech`, sixteen kilohertz mono — through the
//! `decode` it hands in, so the server is sent a few megabytes of speech
//! rather than a film.
//!
//! # The trait is the test seam
//!
//! [`Transcriber`] is what everything above talks to; [`Whisper`] is the real
//! one, and no test reaches a real server.

mod error;
mod run;
mod whisper;

use crate::credentials::{Environment, Settings, SettingsError};

pub use error::TranscribeError;
pub use run::{Asked, Corrected, Transcribed, correct, recording, transcribe};
pub use whisper::{Heard, Language, Transcriber, Whisper};

/// Where the server answers unless told otherwise: what
/// `deploy/whisper/compose.yaml` publishes on this machine.
pub const DEFAULT_ADDRESS: &str = "http://127.0.0.1:8970";

/// The environment variable that says where it answers instead.
pub const VARIABLE: &str = "SCORSESE_WHISPER_URL";

/// The model the image bakes in, by its hub id: Whisper large-v3-turbo
/// converted for faster-whisper.
pub const MODEL: &str = "deepdml/faster-whisper-large-v3-turbo-ct2";

/// How to start the server, for a message that found it not running.
pub const START_IT: &str = "start it from a scorsese checkout with `docker compose --file \
deploy/whisper/compose.yaml up --detach` (the first build bakes a 1.6 GB model in), or point \
SCORSESE_WHISPER_URL, or `whisper_url` in the settings file, at where it runs";

/// Where the server answers, out of exactly these two values: the
/// [`VARIABLE`] first, then the settings file's `whisper_url`, then
/// [`DEFAULT_ADDRESS`] — the order a key resolves in
/// ([`crate::credentials`]).
pub fn address_from(environment: &Environment, settings: &Settings) -> String {
    environment
        .get(VARIABLE)
        .or_else(|| {
            settings
                .whisper_url
                .as_deref()
                .map(str::trim)
                .filter(|url| !url.is_empty())
        })
        .unwrap_or(DEFAULT_ADDRESS)
        .to_owned()
}

/// Where the server answers on this machine: [`address_from`] the process
/// environment, the checkout's `.env` and the settings file. A platform with
/// no settings folder has no setting; a settings file that will not read is
/// said.
pub fn address() -> Result<String, SettingsError> {
    let here = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let settings = match Settings::load() {
        Err(SettingsError::NoConfigDir) => Settings::default(),
        loaded => loaded?,
    };
    Ok(address_from(&Environment::discover(&here), &settings))
}
