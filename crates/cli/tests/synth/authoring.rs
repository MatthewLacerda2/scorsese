//! Starting a recipe, and refusing one that is not a recipe.

use crate::common::{holds, new_project, reload, run_in, temp_dir};
use scorsese_core::AssetKind;

#[test]
fn new_writes_a_recipe_and_the_asset_that_points_at_it() {
    let dir = new_project("synth-new");
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();

    assert!(holds(&dir, "recipes/theme.json"), "the recipe is written");
    let project = reload(&dir);
    let asset = project.assets.first().expect("one asset");
    assert_eq!(asset.kind, AssetKind::SynthAudio);
    assert_eq!(
        asset.recipe.as_ref().map(|r| r.as_str()),
        Some("recipes/theme.json")
    );
    assert!(asset.needs_generation(), "it starts as a sketch");
    assert!(asset.path.is_none(), "and has no media yet");
}

/// `recipes/` is the one project directory whose contents cannot be rebuilt,
/// so a second `new` under a taken name must not overwrite the first.
#[test]
fn a_repeated_name_is_suffixed_rather_than_overwriting() {
    let dir = new_project("synth-collide");
    run_in(&dir, &["synth", "new", "theme"]).ok();
    let first = std::fs::read(dir.join("recipes/theme.json")).expect("the first recipe");

    run_in(&dir, &["synth", "new", "theme"]).ok();
    assert!(holds(&dir, "recipes/theme-2.json"), "the second is renamed");
    assert_eq!(
        std::fs::read(dir.join("recipes/theme.json")).expect("still there"),
        first,
        "the first recipe is untouched"
    );
    assert_eq!(reload(&dir).assets.len(), 2);
}

/// A starter that renders silence would teach nothing and cost an iteration to
/// discover, so both of them have to make a sound as written.
#[test]
fn both_starters_bake_audible_sound_unedited() {
    for kind in ["patch", "song"] {
        let dir = new_project(&format!("synth-starter-{kind}"));
        run_in(&dir, &["synth", "new", "sound", "--kind", kind]).ok();
        run_in(&dir, &["synth", "bake"]).ok();

        let project = reload(&dir);
        let asset = project.assets.first().expect("one asset");
        let path = asset.path.as_ref().expect("baked media");
        let wav = std::fs::read(path.resolve(&dir)).expect("the baked file");
        assert!(
            wav.len() > 44 * 100,
            "the {kind} starter baked almost nothing: {} bytes",
            wav.len()
        );
        assert!(
            wav[44..].iter().any(|&byte| byte != 0),
            "the {kind} starter baked silence"
        );
    }
}

#[test]
fn check_reports_what_a_recipe_is_without_baking_it() {
    let dir = new_project("synth-check");
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    let recipe = dir.join("recipes/theme.json");

    run_in(&dir, &["synth", "check", recipe.to_str().expect("utf-8")])
        .ok()
        .says("song");
    // `generated/` is there from `new`; what matters is that nothing landed in
    // it, since parsing is the whole of what `check` is for.
    let generated = std::fs::read_dir(dir.join("generated")).expect("read generated/");
    assert_eq!(generated.count(), 0, "check must not render anything");
}

#[test]
fn check_fails_on_a_document_that_is_not_a_recipe() {
    let dir = temp_dir("synth-check-bad");
    let recipe = dir.join("nonsense.json");
    std::fs::write(&recipe, r#"{ "recipe": "sonata", "bpm": 120 }"#).expect("write");

    let run = run_in(&dir, &["synth", "check", recipe.to_str().expect("utf-8")]);
    assert!(run.failed, "an unknown recipe kind is a failure");
    run.says("sonata");
}
