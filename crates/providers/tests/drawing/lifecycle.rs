//! Sketch to generated, in one call, and what happens to the stills that do
//! not make it.

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, generate};

use crate::mock::Mock;
use crate::sketched;

#[test]
fn a_sketch_becomes_a_png_in_one_call_with_no_ticket() {
    let (dir, mut project, id) = sketched("drawn", "a lighthouse at dusk");
    let provider = Mock::willing();

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    let Outcome::Generated { path, .. } = &run[0].1 else {
        panic!("{:?}", run[0].1);
    };
    assert!(path.as_str().ends_with(".jpg"), "{path}");
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(asset.state, Some(GenerationState::Generated));
    assert_eq!(asset.path.as_ref(), Some(path));
    assert_eq!(asset.operation, None, "nothing is in flight, so no ticket");
    assert_eq!(asset.queued_at, None);
    assert_eq!(
        asset.estimated_cost_cents,
        Some(11),
        "2K on flash: 10.1¢ and input"
    );
    assert!(path.resolve(&dir).is_file());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_still_with_no_prompt_does_not_stop_the_others() {
    let (dir, mut project, _) = sketched("mixed", "this one is ready");
    project.assets.push(Asset::sketch(
        AssetId::new("later"),
        AssetKind::GeneratedImage,
        "  ",
    ));
    let provider = Mock::willing();

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(matches!(run[0].1, Outcome::Generated { .. }));
    assert!(
        matches!(run[1].1, Outcome::Incomplete { .. }),
        "{:?}",
        run[1].1
    );
    assert_eq!(provider.requests(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_refusal_leaves_the_sketch_to_be_edited() {
    let (dir, mut project, id) = sketched("refused", "something it will not draw");
    let provider = Mock::refusing("I can't draw that.");

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert_eq!(
        run[0].1,
        Outcome::Failed {
            message: String::from("I can't draw that.")
        }
    );
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(asset.state, Some(GenerationState::Sketch));
    assert_eq!(asset.estimated_cost_cents, None, "nothing was drawn");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_ceiling_stops_the_run_before_anything_is_sent() {
    let (dir, mut project, _) = sketched("ceiling", "an expensive still");
    let provider = Mock::willing();

    let refused = generate(&mut project, &dir, &provider, Budget::new(5, 0));

    assert!(refused.is_err(), "11¢ does not fit under a 5¢ ceiling");
    assert_eq!(provider.requests(), 0);
    std::fs::remove_dir_all(&dir).ok();
}
