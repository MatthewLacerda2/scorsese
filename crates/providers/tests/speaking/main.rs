//! The generated-speech lifecycle, driven end to end against a scripted
//! provider.
//!
//! **Nothing here reaches a network or spends a cent**, and that is the point
//! of `SpeechProvider` being a trait rather than a struct: the whole lifecycle
//! — speak, cache, refuse, skip — is exercised without ElevenLabs existing.

#[path = "../common/mod.rs"]
mod common;

mod cache;
mod lifecycle;
mod mock;
mod plans;
mod pricing;
mod refusals;

use scorsese_core::{Asset, AssetId, AssetKind, Project, SpeechRequest};

use common::project;

/// A project with one sketched line in it, and the directory it lives in.
fn sketched(label: &str, line: &str) -> (std::path::PathBuf, Project, AssetId) {
    let (dir, mut project) = project(label);
    let id = AssetId::new("vo");
    let mut asset = Asset::sketch(id.clone(), AssetKind::GeneratedAudio, line);
    asset.speech = Some(SpeechRequest {
        voice_id: Some(String::from("a-voice")),
        ..SpeechRequest::default()
    });
    project.assets.push(asset);
    (dir, project, id)
}

/// The asset with this id, mutably.
fn asset_mut<'a>(project: &'a mut Project, id: &AssetId) -> &'a mut Asset {
    project
        .assets
        .iter_mut()
        .find(|asset| &asset.id == id)
        .expect("the asset is in the table")
}
