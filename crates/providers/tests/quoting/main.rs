//! Quote first, spend second — the rules a token is held to.
//!
//! Every paid surface without a terminal (the MCP tools, the hosted server)
//! answers a first call with a quote and a token and spends only on a second
//! call that hands the token back. What is pinned here is what that token is
//! bound to: exactly the briefs that would be paid for, once, for fifteen
//! minutes. Nothing here reaches a network — a quote needs no key by design.

#[path = "../common/mod.rs"]
mod common;

mod briefs;
mod tokens;

use scorsese_core::{Asset, AssetId, AssetKind, Project, SpeechRequest};

use common::project;

/// A project with one sketched shot and one sketched line with a voice.
fn sketched(label: &str) -> (std::path::PathBuf, Project) {
    let (dir, mut project) = project(label);
    project.assets.push(Asset::sketch(
        AssetId::new("shot"),
        AssetKind::GeneratedVideo,
        "a boat at dawn",
    ));
    let mut line = Asset::sketch(AssetId::new("vo"), AssetKind::GeneratedAudio, "Hello.");
    line.speech = Some(SpeechRequest {
        voice_id: Some(String::from("a-voice")),
        ..SpeechRequest::default()
    });
    project.assets.push(line);
    (dir, project)
}
