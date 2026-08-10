//! What a run would do, read before any run spends: the brief, never the
//! asset's recorded state.

use scorsese_core::GenerationState;
use scorsese_providers::credentials::Budget;
use scorsese_providers::video::{Plan, generate, pending, plan};

use crate::mock::{Answer, Mock};
use crate::sketched;

/// The regression this file exists for. A shot generates, its prompt is
/// edited, and the previous generation's file is still on disk at the path the
/// asset records. Consulting that path reads as *nothing to do* — which is how
/// `scorsese generate` printed "No generated assets in this project." over a
/// project with a stale shot in it and submitted nothing.
#[test]
fn an_edited_brief_with_its_old_file_present_is_pending() {
    let (dir, mut project, id) = sketched("plan-stale", "a red balloon");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"ONE".to_vec())]);

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("submit");
    assert!(pending(&project, &dir), "a shot in flight is still work");
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("collect");
    assert!(!pending(&project, &dir), "a realised brief is not work");

    let asset = project
        .assets
        .iter_mut()
        .find(|asset| asset.id == id)
        .expect("the asset");
    asset.prompt = Some(String::from("a blue balloon"));
    asset.state = Some(GenerationState::Stale);

    let old = project
        .asset(&id)
        .and_then(|asset| asset.path.clone())
        .expect("the old generation's path");
    assert!(old.resolve(&dir).is_file(), "the old file is still on disk");
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(
        plan(&project, &dir, asset),
        Plan::Submit,
        "it would be sent"
    );
    assert!(
        pending(&project, &dir),
        "the old file must not hide the stale shot"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The other direction the recorded state can lie in: the document says
/// generated but the file is gone — deleted, or never copied along with the
/// project — and that one is work again whatever the state field claims.
#[test]
fn a_realised_brief_whose_file_was_deleted_is_pending_again() {
    let (dir, mut project, id) = sketched("plan-lost", "a red balloon");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"ONE".to_vec())]);

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("submit");
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("collect");
    let path = project
        .asset(&id)
        .and_then(|asset| asset.path.clone())
        .expect("the generation's path");
    std::fs::remove_file(path.resolve(&dir)).expect("delete the generation");

    assert!(pending(&project, &dir), "a lost file is work again");
    std::fs::remove_dir_all(&dir).ok();
}
