//! What a spoken line asks for, and what the vendor will not honour.
//!
//! The first test here is the one that matters most, and it is the odd one out
//! in this whole suite: it checks that we refuse a request the vendor would
//! have **accepted**. Every other refusal in validation saves a round trip; that
//! one saves a narration nobody would have known was wrong.

use crate::common::{assert_only_problem, asset_id, asset_mut, project};
use scorsese_core::{
    AssetField as F, AssetKind, AssetProblem as E, MAX_CHARACTERS, Project, SpeechModel,
    SpeechProblem as S, SpeechRequest,
};

/// The fixture's narration asset, with `edit` applied to its request.
fn saying(edit: impl FnOnce(&mut SpeechRequest)) -> Project {
    let mut p = project();
    let mut request = SpeechRequest::default();
    edit(&mut request);
    asset_mut(&mut p, "vo-open").speech = Some(request);
    p
}

#[test]
fn a_language_the_model_would_drop_is_refused() {
    let p = saying(|r| {
        r.model = SpeechModel::Standard;
        r.language = Some("pt".to_owned());
    });
    assert_only_problem(
        &p,
        E::Speech(S::LanguageIgnored {
            asset: asset_id("vo-open"),
            model: "standard",
            language: "pt".to_owned(),
        }),
    );
}

#[test]
fn the_models_that_honour_a_language_take_one() {
    for model in [SpeechModel::Expressive, SpeechModel::Fast] {
        let p = saying(|r| {
            r.model = model;
            r.language = Some("pt".to_owned());
        });
        assert!(
            p.validate().is_ok(),
            "{} should take a language",
            model.as_str()
        );
    }
}

#[test]
fn no_language_is_fine_on_every_model() {
    for model in SpeechModel::ALL {
        let p = saying(|r| r.model = model);
        assert!(p.validate().is_ok(), "{} with no language", model.as_str());
    }
}

#[test]
fn a_line_longer_than_the_vendor_speaks_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "vo-open").prompt = Some("a".repeat(MAX_CHARACTERS + 1));
    assert_only_problem(
        &p,
        E::Speech(S::TooLong {
            asset: asset_id("vo-open"),
            found: MAX_CHARACTERS + 1,
            max: MAX_CHARACTERS,
        }),
    );
}

/// Accented letters are two bytes each, so a byte count would refuse a script
/// a quarter short of the limit — and would have priced it wrong as well.
#[test]
fn the_limit_counts_characters_and_not_bytes() {
    let mut p = project();
    let line = "ç".repeat(MAX_CHARACTERS);
    assert!(line.len() > MAX_CHARACTERS, "the fixture must be 2-byte");
    asset_mut(&mut p, "vo-open").prompt = Some(line);
    assert!(p.validate().is_ok(), "exactly the limit, in characters");
}

#[test]
fn an_empty_voice_looks_chosen_and_is_not() {
    let p = saying(|r| r.voice_id = Some(String::new()));
    assert_only_problem(
        &p,
        E::Speech(S::BlankVoice {
            asset: asset_id("vo-open"),
        }),
    );
}

/// A sketch nobody has finished is a legitimate document. The refusal for a
/// missing voice belongs at the moment of spending, not the moment of writing.
#[test]
fn a_narration_with_no_voice_yet_is_a_legitimate_document() {
    let p = saying(|r| r.voice_id = None);
    assert!(p.validate().is_ok());
}

#[test]
fn a_request_on_a_kind_that_takes_none_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "shot-city").speech = Some(SpeechRequest::default());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("shot-city"),
            field: F::Speech,
            kind: AssetKind::GeneratedVideo,
        },
    );
}
