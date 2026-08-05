//! The generated-video lifecycle, driven end to end against a scripted
//! provider.
//!
//! **Nothing here reaches a network or spends a cent**, and that is the point
//! of `VideoProvider` being a trait rather than a struct: the whole lifecycle —
//! submit, wait, resume, refuse, cache — is exercised without Veo existing.

#[path = "../common/mod.rs"]
mod common;

mod cache;
mod lifecycle;
mod mock;
mod refusals;

use scorsese_core::{Asset, AssetId, AssetKind, Project};

use common::project;

/// A project with one sketched shot in it, and the directory it lives in.
fn sketched(label: &str, prompt: &str) -> (std::path::PathBuf, Project, AssetId) {
    let (dir, mut project) = project(label);
    let id = AssetId::new("shot");
    project
        .assets
        .push(Asset::sketch(id.clone(), AssetKind::GeneratedVideo, prompt));
    (dir, project, id)
}

/// What an asset's state, path and ticket are, in one line a test can assert
/// against.
fn standing(project: &Project, id: &AssetId) -> (String, Option<String>, Option<String>) {
    let asset = project.asset(id).expect("the asset is in the table");
    (
        format!("{:?}", asset.state),
        asset.path.as_ref().map(|path| path.as_str().to_owned()),
        asset.operation.clone(),
    )
}
