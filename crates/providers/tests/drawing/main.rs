//! The generated-still lifecycle, driven end to end against a scripted
//! provider.
//!
//! **Nothing here reaches a network or spends a cent** — `ImageProvider` is a
//! trait so the whole lifecycle is exercised without Gemini existing.

#[path = "../common/mod.rs"]
mod common;

mod cache;
mod lifecycle;
mod mock;
mod pricing;
mod references;

use scorsese_core::{Asset, AssetId, AssetKind, Project};

use common::project;

/// A project with one sketched still in it, and the directory it lives in.
fn sketched(label: &str, prompt: &str) -> (std::path::PathBuf, Project, AssetId) {
    let (dir, mut project) = project(label);
    let id = AssetId::new("poster");
    project
        .assets
        .push(Asset::sketch(id.clone(), AssetKind::GeneratedImage, prompt));
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
