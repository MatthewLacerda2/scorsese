//! What a line is worked out to cost, and the ceiling that stops a run.

use scorsese_core::{Asset, AssetId, AssetKind, SpeechModel, SpeechRequest};
use scorsese_providers::credentials::Budget;
use scorsese_providers::prices::speech;
use scorsese_providers::speech::generate;

use crate::speaking::mock::Mock;
use crate::speaking::sketched;

/// The vendor bills by character, so the figure is exact before anything is
/// sent — the one place scorsese knows a price rather than estimating a length.
#[test]
fn a_thousand_characters_costs_the_published_rate() {
    for (model, cents) in [
        (SpeechModel::Expressive, 8),
        (SpeechModel::Standard, 8),
        (SpeechModel::Fast, 4),
    ] {
        let estimate = speech(model, 1000).expect("a published rate");
        assert_eq!(estimate.cents, cents, "{}", model.as_str());
        assert_eq!(estimate.characters, 1000);
    }
}

/// **Rounds up.** Unlike video this does not land on whole cents, and rounding
/// down would let a run slip past the ceiling a fraction of a cent at a time.
/// A ceiling that can be crossed a little at a time is not a ceiling.
#[test]
fn a_part_of_a_cent_rounds_up_and_never_down() {
    // 137 characters at 8c/1000 is 1.096 cents.
    assert_eq!(speech(SpeechModel::Standard, 137).expect("a rate").cents, 2);
    // One character is a tiny fraction of a cent, and still not free.
    assert_eq!(speech(SpeechModel::Standard, 1).expect("a rate").cents, 1);
    // Nothing to say costs nothing.
    assert_eq!(speech(SpeechModel::Standard, 0).expect("a rate").cents, 0);
    // Exactly on the boundary does not creep up.
    assert_eq!(speech(SpeechModel::Fast, 2000).expect("a rate").cents, 8);
}

/// Every model scorsese offers has a published rate. `UnpricedSpeech` exists so
/// that adding one without a price is a refusal somebody has to answer for
/// rather than a zero nobody notices — this is what keeps it unreachable.
#[test]
fn no_model_scorsese_offers_is_unpriced() {
    for model in SpeechModel::ALL {
        assert!(speech(model, 100).is_ok(), "{} has no rate", model.as_str());
    }
}

/// A ceiling that admits one line must refuse the second — checked against what
/// the run has already committed, not against what the project had spent before
/// it started.
#[test]
fn the_ceiling_counts_what_this_run_has_already_committed() {
    let (dir, mut project, id) = sketched("ceiling", &"a".repeat(1000));
    let mut second = Asset::sketch(
        AssetId::new("vo-second"),
        AssetKind::GeneratedAudio,
        "b".repeat(1000),
    );
    // Both lines name their model rather than taking the default, so the
    // arithmetic below is the test's own and does not move when the default does.
    let dearer = SpeechRequest {
        model: SpeechModel::Standard,
        voice_id: Some(String::from("a-voice")),
        ..SpeechRequest::default()
    };
    second.speech = Some(dearer.clone());
    crate::speaking::asset_mut(&mut project, &id).speech = Some(dearer);
    project.assets.push(second);
    let provider = Mock::willing();

    // Two 1000-character lines at the standard rate are 8 cents each. 15
    // admits one and cannot admit two.
    let refused = generate(&mut project, &dir, &provider, Budget::new(15, 0))
        .expect_err("two lines at 8 cents is over a 15-cent ceiling");

    assert!(refused.to_string().contains("ceiling"), "{refused}");
    assert_eq!(
        provider.requests(),
        1,
        "the first line fits and the second must not"
    );
    // The line spoken before the stop is reported with it, not dropped (#1153).
    let spoken: Vec<&str> = refused.run.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(spoken, [id.as_str()]);
    let unsent: Vec<&str> = refused.unsent.iter().map(AssetId::as_str).collect();
    assert_eq!(unsent, ["vo-second"]);
    let says = refused.says();
    assert!(
        says.contains("1 line was spoken, 1 line was not (vo-second)"),
        "{says}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The ceiling is checked before the call that spends anything, never after.
#[test]
fn a_run_over_the_ceiling_sends_nothing() {
    let (dir, mut project, _) = sketched("overbudget", &"a".repeat(5000));
    let provider = Mock::willing();

    let refused = generate(&mut project, &dir, &provider, Budget::new(10, 0))
        .expect_err("20 cents is over a 10-cent ceiling");

    assert!(refused.to_string().contains("ceiling"), "{refused}");
    assert_eq!(provider.requests(), 0, "the check must come first");
    std::fs::remove_dir_all(&dir).ok();
}
