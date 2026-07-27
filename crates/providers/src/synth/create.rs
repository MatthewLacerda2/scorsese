//! Starting a synth asset: a recipe on disk, and a row pointing at it.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath, RECIPES_DIR, asset_id_for};

use super::error::SynthesisError;
use super::recipe::Recipe;
use super::starter::Starter;

/// Writes a starter recipe into `recipes/` and adds the `synth_audio` asset
/// that points at it, in `sketch`.
///
/// The id and the file name come from `name`, sanitised and made unique, so
/// `scorsese synth new theme` twice gives `theme` and `theme-2` rather than
/// one silently overwriting the other's recipe. Overwriting would lose
/// authored work, which `recipes/` is precisely the directory that cannot
/// afford.
///
/// The project is left describing the new asset; saving it is the caller's.
pub fn create(
    project: &mut Project,
    project_root: &Path,
    name: &str,
    starter: Starter,
) -> Result<AssetId, SynthesisError> {
    let id = asset_id_for(project, name);
    let relative = ProjectPath::new(format!("{RECIPES_DIR}/{id}.json"));
    let file = relative.resolve(project_root);

    let json = starter
        .recipe()
        .to_json()
        .expect("a starter recipe serialises");
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SynthesisError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(&file, json).map_err(|source| SynthesisError::Write {
        path: file.clone(),
        source,
    })?;

    project
        .assets
        .push(Asset::synth(id.clone(), AssetKind::SynthAudio, relative));
    Ok(id)
}

/// Parses a recipe and checks that it describes something renderable, without
/// rendering it.
///
/// Worth its own entry point because parsing is instant and rendering is not:
/// an agent iterating on a song wants a malformed document reported in
/// milliseconds, not after a bake it was never going to keep.
pub fn check(json: &str) -> Result<Recipe, serde_json::Error> {
    Recipe::from_json(json)
}
