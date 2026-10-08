//! An unchanged brief is never drawn twice; a changed one is a new brief.

use scorsese_core::{GenerationState, ImageRequest, ImageResolution, ImageThinking};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, generate};

use crate::mock::Mock;
use crate::{asset_mut, sketched};

#[test]
fn a_second_run_draws_nothing() {
    let (dir, mut project, _) = sketched("cached", "a lighthouse at dusk");
    let provider = Mock::willing();

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let again = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(
        matches!(again[0].1, Outcome::Cached { .. }),
        "{:?}",
        again[0].1
    );
    assert_eq!(provider.requests(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_note_is_not_part_of_the_brief() {
    let (dir, mut project, id) = sketched("noted", "a lighthouse at dusk");
    let provider = Mock::willing();
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    asset_mut(&mut project, &id).note = Some(String::from("the opening backdrop"));
    let again = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(matches!(again[0].1, Outcome::Cached { .. }));
    assert_eq!(provider.requests(), 1, "a note is handed to nobody");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn naming_the_default_size_is_the_same_brief_and_another_size_is_not() {
    let (dir, mut project, id) = sketched("sized", "a lighthouse at dusk");
    let provider = Mock::willing();
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    asset_mut(&mut project, &id).image = Some(ImageRequest {
        resolution: Some(ImageResolution::K2),
        ..ImageRequest::default()
    });
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert_eq!(
        provider.requests(),
        1,
        "2K is what the default model draws by default"
    );

    asset_mut(&mut project, &id).image = Some(ImageRequest {
        resolution: Some(ImageResolution::K4),
        ..ImageRequest::default()
    });
    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert!(matches!(run[0].1, Outcome::Generated { .. }));
    assert_eq!(provider.requests(), 2);
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(asset.state, Some(GenerationState::Generated));
    std::fs::remove_dir_all(&dir).ok();
}

/// A thinking level is part of the brief only when it is not the model's own:
/// naming `medium` on 2.1 is what was drawn, `high` is a new picture.
#[test]
fn naming_the_default_thinking_is_the_same_brief_and_another_level_is_not() {
    let (dir, mut project, id) = sketched("thought", "a lighthouse at dusk");
    let provider = Mock::willing();
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    let thinking = |level| ImageRequest {
        thinking: Some(level),
        ..ImageRequest::default()
    };
    asset_mut(&mut project, &id).image = Some(thinking(ImageThinking::Medium));
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert_eq!(provider.requests(), 1, "medium is 2.1's own default");

    asset_mut(&mut project, &id).image = Some(thinking(ImageThinking::High));
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert_eq!(provider.requests(), 2);
    std::fs::remove_dir_all(&dir).ok();
}
