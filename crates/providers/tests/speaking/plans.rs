//! What a run would do, read before any run spends: the brief, never the
//! asset's recorded state.

use scorsese_providers::credentials::Budget;
use scorsese_providers::speech::{Plan, generate, pending, plan};

use crate::mock::Mock;
use crate::{asset_mut, sketched};

/// The narration shape of the video regression: a rewritten line whose
/// previous reading is still on disk at the recorded path. Consulting that
/// path reads as *nothing to do*, and the rewrite would never be spoken.
#[test]
fn a_rewritten_line_with_its_old_file_present_is_pending() {
    let (dir, mut project, id) = sketched("plan-stale", "the first take");
    let provider = Mock::willing();

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert!(!pending(&project, &dir), "a spoken line is not work");

    asset_mut(&mut project, &id).prompt = Some(String::from("the second take"));

    let old = project
        .asset(&id)
        .and_then(|asset| asset.path.clone())
        .expect("the old reading's path");
    assert!(old.resolve(&dir).is_file(), "the old file is still on disk");
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(plan(&dir, asset), Plan::Submit, "it would be spoken");
    assert!(
        pending(&project, &dir),
        "the old file must not hide the rewrite"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// An incomplete line counts as work: the pass is where *not yet* is voiced,
/// naming the line and what it is missing.
#[test]
fn a_line_with_no_voice_yet_is_pending() {
    let (dir, mut project, id) = sketched("plan-voiceless", "a line");
    asset_mut(&mut project, &id).speech = None;

    let asset = project.asset(&id).expect("the asset");
    assert!(
        matches!(plan(&dir, asset), Plan::Unready(_)),
        "no voice chosen means the line cannot be sent yet"
    );
    assert!(
        pending(&project, &dir),
        "the run is where 'not yet' is said"
    );
    std::fs::remove_dir_all(&dir).ok();
}
