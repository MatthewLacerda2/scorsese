//! The synthesis kind: its brief is a recipe, and only a recipe.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{AssetField as F, AssetKind, AssetProblem as E, GenerationState, ProjectPath};

#[test]
fn a_synth_asset_needs_a_recipe() {
    let mut p = project();
    asset_mut(&mut p, "score").recipe = None;
    assert_only_problem(
        &p,
        E::MissingField {
            asset: asset_id("score"),
            field: F::Recipe,
            kind: AssetKind::SynthAudio,
        },
    );
}

#[test]
fn a_recipe_obeys_the_project_path_rules() {
    let mut p = project();
    let escaping = ProjectPath::new("../elsewhere/theme.json");
    asset_mut(&mut p, "score").recipe = Some(escaping.clone());
    assert_only_problem(
        &p,
        E::BadRecipePath {
            asset: asset_id("score"),
            path: escaping,
            problem: scorsese_core::PathProblem::ParentEscape,
        },
    );
}

#[test]
fn a_recipe_belongs_to_no_other_kind() {
    let mut p = project();
    asset_mut(&mut p, "logo").recipe = Some(ProjectPath::new("recipes/theme.json"));
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("logo"),
            field: F::Recipe,
            kind: AssetKind::Image,
        },
    );
}

/// The two briefs are alternatives, not a pair: a synth asset that also
/// carries a prompt has been described twice, and only one of the two would
/// ever be read.
#[test]
fn a_synth_asset_carries_no_prompt() {
    let mut p = project();
    asset_mut(&mut p, "score").prompt = Some("something cinematic".to_owned());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("score"),
            field: F::Prompt,
            kind: AssetKind::SynthAudio,
        },
    );
}

#[test]
fn a_synth_asset_needs_a_state() {
    let mut p = project();
    asset_mut(&mut p, "score").state = None;
    assert_only_problem(
        &p,
        E::MissingField {
            asset: asset_id("score"),
            field: F::State,
            kind: AssetKind::SynthAudio,
        },
    );
}

/// A sketch has no file yet and is not supposed to: the mix contributes
/// silence for it, exactly as it does for an ungenerated narration prompt.
#[test]
fn a_sketch_recipe_needs_no_file_yet() {
    let p = project();
    let score = p.asset(&asset_id("score")).expect("asset");
    assert!(score.needs_generation());
    assert!(!score.has_renderable_media());
    assert!(problems(&p).is_empty());
}

/// Claiming the media exists without saying where is the one lifecycle
/// mistake a hand-edited state can make, and it is caught for a recipe the
/// same way it is for a prompt.
#[test]
fn a_generated_recipe_must_say_where_the_file_is() {
    let mut p = project();
    asset_mut(&mut p, "score").state = Some(GenerationState::Generated);
    assert_only_problem(
        &p,
        E::GeneratedWithoutPath {
            asset: asset_id("score"),
        },
    );
}

#[test]
fn synthesis_is_generated_but_not_prompted() {
    let kind = AssetKind::SynthAudio;
    assert!(
        kind.is_generated(),
        "it has a lifecycle and lands in generated/"
    );
    assert!(
        !kind.is_prompted(),
        "its brief is a document, not a sentence"
    );
    assert!(kind.is_synthesized());
    assert!(kind.is_audible(), "it belongs on an audio track");
    assert!(!kind.is_visual());
}
