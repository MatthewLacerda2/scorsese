//! Baking: the cache, the rebake an edited recipe forces, and what a bake
//! costs.

use std::path::Path;

use crate::common::{new_project, reload, run_in};
use scorsese_core::GenerationState;

/// Starts a project with one song recipe already baked.
fn baked(label: &str) -> std::path::PathBuf {
    let dir = new_project(label);
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    run_in(&dir, &["synth", "bake"]).ok();
    dir
}

/// Where the one asset's media is, and how big.
fn media(dir: &Path) -> (String, usize) {
    let project = reload(dir);
    let asset = project.assets.first().expect("one asset");
    assert_eq!(asset.state, Some(GenerationState::Generated));
    let path = asset.path.as_ref().expect("baked media");
    let bytes = std::fs::read(path.resolve(dir)).expect("the baked file");
    (path.as_str().to_owned(), bytes.len())
}

#[test]
fn baking_generates_the_media_and_records_it() {
    let dir = baked("synth-bake");
    let (path, bytes) = media(&dir);
    assert!(path.starts_with("generated/"), "landed in {path}");
    assert!(path.ends_with(".wav"));
    assert!(bytes > 44, "a header and some samples");

    let project = reload(&dir);
    let asset = project.assets.first().expect("one asset");
    assert!(asset.has_renderable_media());
    assert!(!asset.needs_generation(), "nothing left to do");
    assert!(asset.sha256.is_some(), "the finished file is hashed");
}

/// Re-running is the thing an agent does by accident, so it has to be free.
#[test]
fn baking_twice_renders_nothing_the_second_time() {
    let dir = baked("synth-cache");
    run_in(&dir, &["synth", "bake"])
        .ok()
        .says("0 rendered, 1 cached");
}

/// The whole point of hashing the recipe rather than trusting `state`: nobody
/// had to mark this stale, and it still got redone.
#[test]
fn editing_a_recipe_rebakes_it_without_being_told() {
    let dir = baked("synth-stale");
    let (before, _) = media(&dir);

    let recipe = dir.join("recipes/theme.json");
    let edited = std::fs::read_to_string(&recipe)
        .expect("the recipe")
        .replace("\"bpm\": 96.0", "\"bpm\": 140.0");
    std::fs::write(&recipe, edited).expect("write the edit");

    run_in(&dir, &["synth", "bake"]).ok().says("1 rendered");
    let (after, _) = media(&dir);
    assert_ne!(before, after, "a new recipe means a new file");
}

/// Content addressing, from the outside: the same recipe under two names is
/// one file on disk, because the name is the hash of what is in it.
#[test]
fn two_assets_with_identical_recipes_share_one_bake() {
    let dir = new_project("synth-shared");
    run_in(&dir, &["synth", "new", "one"]).ok();
    run_in(&dir, &["synth", "new", "two"]).ok();
    run_in(&dir, &["synth", "bake"]).ok();

    let project = reload(&dir);
    let paths: Vec<_> = project
        .assets
        .iter()
        .map(|asset| asset.path.clone().expect("baked"))
        .collect();
    assert_eq!(paths[0], paths[1], "identical recipes, identical bake");
    let generated = std::fs::read_dir(dir.join("generated")).expect("read generated/");
    assert_eq!(generated.count(), 1, "and only one file was written");
}

#[test]
fn baking_one_asset_by_id_leaves_the_others_alone() {
    let dir = new_project("synth-one");
    run_in(&dir, &["synth", "new", "kept", "--kind", "song"]).ok();
    run_in(&dir, &["synth", "new", "skipped"]).ok();
    run_in(&dir, &["synth", "bake", "kept"]).ok();

    let project = reload(&dir);
    let state = |id: &str| {
        project
            .assets
            .iter()
            .find(|asset| asset.id.as_str() == id)
            .expect("the asset")
            .state
    };
    assert_eq!(state("kept"), Some(GenerationState::Generated));
    assert_eq!(state("skipped"), Some(GenerationState::Sketch));
}

#[test]
fn baking_an_id_that_is_not_there_says_so() {
    let dir = new_project("synth-missing");
    let run = run_in(&dir, &["synth", "bake", "nope"]);
    assert!(run.failed);
    run.says("nope");
}

#[test]
fn a_project_with_no_recipes_says_where_to_start() {
    let dir = new_project("synth-none");
    run_in(&dir, &["synth", "bake"]).ok().says("synth new");
}
