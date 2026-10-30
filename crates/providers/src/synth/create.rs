//! Starting a synth asset: a recipe on disk, and a row pointing at it.

use std::path::Path;

use scorsese_core::{
    Asset, AssetId, AssetKind, Project, ProjectPath, RECIPES_DIR, asset_id_for, write,
};

use scorsese_zimmer::Patch;

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
    start(project, project_root, name, &starter.recipe())
}

/// Writes `recipe` into `recipes/` under a name made from `name`, and adds the
/// `synth_audio` asset that points at it, in `sketch`.
///
/// The one way a synth asset comes to exist, whoever wrote the document: a
/// starter and an imported MIDI file land exactly alike, so nothing downstream
/// can tell — or needs to — which one it is holding.
pub(super) fn start(
    project: &mut Project,
    project_root: &Path,
    name: &str,
    recipe: &Recipe,
) -> Result<AssetId, SynthesisError> {
    let id = asset_id_for(project, name);
    let relative = ProjectPath::new(format!("{RECIPES_DIR}/{id}.json"));
    let file = super::place(project_root, &relative)?;

    let json = recipe.to_json().expect("a recipe serialises");
    // Atomic: a recipe is authored work, and a half-written one is the kind
    // `recipes/` cannot afford to lose. Never through a link out of the
    // project.
    write::in_project(project_root, &relative, json)
        .map_err(|source| SynthesisError::Write { path: file, source })?;

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

/// Whether a document in `recipes/` is a bare patch file rather than a recipe.
///
/// Told by the one key that separates them: a recipe carries a `recipe` tag
/// and a patch file never can, since a patch refuses every field it does not
/// know. So this is not guessing from the keys present, which the recipe
/// format rules out: anything that is not a JSON object, or that has the tag,
/// is a recipe, and parsing it as one says what is wrong.
pub fn is_patch_file(json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|value| {
            value
                .as_object()
                .map(|object| !object.contains_key("recipe"))
        })
        .unwrap_or(false)
}

/// Checks a bare patch file — the kind a song names by path, so several songs
/// share one instrument — by the rules it meets when a song plays it: parsed
/// as the resolver parses it, then held to the bounds every note is.
pub fn check_patch(json: &str) -> Result<(), String> {
    let patch = Patch::from_json(json).map_err(|error| error.to_string())?;
    patch.validate().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = r#"{ "source": { "kind": "noise" },
        "amp": { "a": 0.0, "d": 0.1, "s": 0.0, "r": 0.0 } }"#;

    /// The tag decides, and only an object can lack one: a recipe, or a
    /// document that is no object at all, is read as a recipe.
    #[test]
    fn a_patch_file_is_an_object_without_the_recipe_tag() {
        assert!(is_patch_file(PATCH));
        assert!(!is_patch_file(r#"{ "recipe": "song" }"#));
        assert!(!is_patch_file("{ nonsense"));
        assert!(!is_patch_file("[]"));
    }

    /// Parsed as a song's resolver parses it, then held to a note's bounds.
    #[test]
    fn a_patch_file_is_checked_by_the_rules_a_song_plays_it_by() {
        assert_eq!(check_patch(PATCH), Ok(()));
        let silent = r#"{ "source": { "kind": "osc_stack", "oscs": [] },
            "amp": { "a": 0.0, "d": 0.1, "s": 0.0, "r": 0.0 } }"#;
        assert!(check_patch(silent).is_err(), "an empty stack cannot sound");
        assert!(check_patch(r#"{ "bpm": 120 }"#).is_err());
    }
}
