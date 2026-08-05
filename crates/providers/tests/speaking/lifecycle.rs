//! Sketch to generated, and what happens to the lines that do not make it.

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, MAX_CHARACTERS, SpeechRequest};
use scorsese_providers::credentials::Budget;
use scorsese_providers::speech::{Outcome, generate};

use crate::mock::{Answer, Mock};
use crate::{asset_mut, sketched};

#[test]
fn a_sketch_becomes_a_generated_file_in_one_call() {
    let (dir, mut project, id) = sketched("spoken", "a line worth hearing");
    let provider = Mock::giving(vec![Answer::Speaks(b"AUDIO".to_vec())]);

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(
        matches!(run[0].1, Outcome::Generated { .. }),
        "{:?}",
        run[0].1
    );
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(asset.state, Some(GenerationState::Generated));
    assert!(asset.path.is_some(), "it points at the file it wrote");
    assert!(asset.sha256.is_some(), "and says what is in it");
    assert!(
        asset.estimated_cost_cents.is_some(),
        "and what it was worked out to cost"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The gap this crate cannot close. A reading is as long as the words take, and
/// measuring an MP3 needs a decoder `scorsese-providers` does not have. So
/// `media` is left for whoever probes — and the commands do. A generated shot
/// arrives the same way, for the same reason: see #249.
#[test]
fn a_spoken_line_arrives_without_a_length() {
    let (dir, mut project, id) = sketched("unmeasured", "how long is this");
    let provider = Mock::willing();

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(
        project.asset(&id).expect("the asset").media.is_none(),
        "nothing here can measure audio; probing is what fills this in"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The heart of the [`Incomplete`] design: a cut being written normally has a
/// line in it nobody has chosen a voice for, and that must not cost the other
/// nineteen their run.
#[test]
fn a_line_with_no_voice_does_not_stop_the_others() {
    let (dir, mut project, _) = sketched("mixed", "this one is ready");
    project.assets.push(Asset::sketch(
        AssetId::new("vo-later"),
        AssetKind::GeneratedAudio,
        "this one is not",
    ));
    let provider = Mock::willing();

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    let ready = &run
        .iter()
        .find(|(id, _)| id.as_str() == "vo")
        .expect("vo")
        .1;
    let later = &run
        .iter()
        .find(|(id, _)| id.as_str() == "vo-later")
        .expect("vo-later")
        .1;
    assert!(matches!(ready, Outcome::Generated { .. }), "{ready:?}");
    assert!(matches!(later, Outcome::Incomplete { .. }), "{later:?}");
    assert_eq!(provider.requests(), 1, "the incomplete line was not sent");
    std::fs::remove_dir_all(&dir).ok();
}

/// Refused by the provider is not the same as not ready, and the asset is left
/// exactly as it was so the brief can be edited and tried again.
#[test]
fn a_refused_line_leaves_the_asset_alone() {
    let (dir, mut project, id) = sketched("refused", "something the vendor dislikes");
    let provider = Mock::giving(vec![Answer::Refuses(String::from("no"))]);

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(matches!(run[0].1, Outcome::Failed { .. }), "{:?}", run[0].1);
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(asset.state, Some(GenerationState::Sketch), "still a sketch");
    assert!(asset.path.is_none(), "and points at nothing");
    std::fs::remove_dir_all(&dir).ok();
}

/// The last check before money moves. Validation reports this too, and that is
/// not a duplicate — validation is a report somebody may not have run.
#[test]
fn a_line_over_the_vendors_limit_is_never_sent() {
    let (dir, mut project, id) = sketched("toolong", "short for now");
    asset_mut(&mut project, &id).prompt = Some("a".repeat(MAX_CHARACTERS + 1));
    let provider = Mock::willing();

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(
        matches!(run[0].1, Outcome::Incomplete { .. }),
        "{:?}",
        run[0].1
    );
    assert_eq!(provider.requests(), 0, "it must never reach the vendor");
    std::fs::remove_dir_all(&dir).ok();
}

/// A voice of nothing but spaces looks chosen and is not.
#[test]
fn a_blank_voice_is_no_voice() {
    let (dir, mut project, id) = sketched("blank", "who says this");
    asset_mut(&mut project, &id).speech = Some(SpeechRequest {
        voice_id: Some(String::from("   ")),
        ..SpeechRequest::default()
    });
    let provider = Mock::willing();

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(
        matches!(run[0].1, Outcome::Incomplete { .. }),
        "{:?}",
        run[0].1
    );
    assert_eq!(provider.requests(), 0);
    std::fs::remove_dir_all(&dir).ok();
}
