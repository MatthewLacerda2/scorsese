//! A recipe is a path, and a path in this project stays inside this project.
//!
//! These are the cases the CLI cannot produce, because `synth new` always
//! writes a well-formed one. They matter because a recipe is the one thing in
//! a project that *names another file*, so it is the one place a hand-edited
//! document could reach somewhere it should not.

use crate::common::{project, synth_asset, write};
use scorsese_core::PathProblem;
use scorsese_providers::synth::{SynthesisError, bake_asset};

const SILENT_PATCH: &str = r#"{
  "recipe": "patch", "note": "C4", "duration": 0.1,
  "patch": { "source": { "kind": "noise" },
             "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } }
}"#;

#[test]
fn a_recipe_climbing_out_of_the_project_is_refused_before_it_is_opened() {
    let (dir, mut project) = project("escape");
    // Written where the path would land, so the refusal cannot be mistaken for
    // "the file was not there".
    let outside = dir.parent().expect("a parent").join("escape.json");
    std::fs::write(&outside, SILENT_PATCH).expect("write the file outside");

    let id = synth_asset(&mut project, "sneak", "../escape.json");
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    assert!(
        matches!(
            refused,
            SynthesisError::BadRecipePath {
                problem: PathProblem::ParentEscape,
                ..
            }
        ),
        "got {refused}"
    );
    std::fs::remove_file(outside).ok();
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_absolute_recipe_path_is_refused() {
    let (dir, mut project) = project("absolute");
    let id = synth_asset(&mut project, "sneak", "/etc/hostname");
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    assert!(
        matches!(
            refused,
            SynthesisError::BadRecipePath {
                problem: PathProblem::Absolute,
                ..
            }
        ),
        "got {refused}"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_recipe_that_is_not_there_says_which_file_is_missing() {
    let (dir, mut project) = project("missing-recipe");
    let id = synth_asset(&mut project, "gone", "recipes/gone.json");
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    assert!(
        matches!(refused, SynthesisError::Read { .. }),
        "got {refused}"
    );
    assert!(refused.to_string().contains("gone.json"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_recipe_that_does_not_parse_says_so_with_its_file() {
    let (dir, mut project) = project("malformed");
    write(&dir, "recipes/broken.json", "{ not json at all");
    let id = synth_asset(&mut project, "broken", "recipes/broken.json");
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    assert!(
        matches!(refused, SynthesisError::Malformed { .. }),
        "got {refused}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A document that parses but describes something unplayable is a different
/// failure from one that does not parse, and an agent repairing it needs to
/// know which.
#[test]
fn a_recipe_that_parses_but_cannot_render_is_a_separate_failure() {
    let (dir, mut project) = project("unrenderable");
    write(
        &dir,
        "recipes/empty.json",
        &SILENT_PATCH.replace(
            r#"{ "kind": "noise" }"#,
            r#"{ "kind": "osc_stack", "oscs": [] }"#,
        ),
    );
    let id = synth_asset(&mut project, "empty", "recipes/empty.json");
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    assert!(
        matches!(refused, SynthesisError::Unrenderable { .. }),
        "got {refused}"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn asking_to_bake_something_that_is_not_synthesised_is_refused() {
    let (dir, mut project) = project("wrong-kind");
    let id = scorsese_core::AssetId::new("shot");
    project.assets.push(scorsese_core::Asset::sketch(
        id.clone(),
        scorsese_core::AssetKind::GeneratedVideo,
        "a city at dawn",
    ));
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    assert!(
        matches!(refused, SynthesisError::NotSynthesised { .. }),
        "got {refused}"
    );
    std::fs::remove_dir_all(dir).ok();
}
