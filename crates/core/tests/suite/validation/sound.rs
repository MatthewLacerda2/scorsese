//! What a generated sound effect asks for (#1265): one block per asset, a
//! length the vendor makes, and nowhere but `generated_audio`.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{
    AssetField as F, AssetKind, AssetProblem as E, AudioBlock, MAX_CHARACTERS, MAX_SOUND_SECONDS,
    MIN_SOUND_SECONDS, MusicRequest, Project, SoundProblem as S, SoundRequest, SpeechRequest,
};

/// The fixture's narration asset turned into a sound effect of `seconds`.
fn sounding(seconds: Option<f64>) -> Project {
    let mut p = project();
    asset_mut(&mut p, "vo-open").sound = Some(SoundRequest {
        seconds,
        ..SoundRequest::default()
    });
    p
}

#[test]
fn a_sound_is_a_generated_audio_asset_with_a_sound_block() {
    let p = sounding(Some(2.5));
    p.validate().expect("a sound validates");
    let asset = p.assets.iter().find(|a| a.id == asset_id("vo-open"));
    assert_eq!(asset.and_then(|a| a.audio_block()), Some(AudioBlock::Sound));
    assert!(sounding(None).validate().is_ok(), "a sketch with no length");
}

#[test]
fn a_sound_plays_once_unless_it_says_otherwise() {
    assert!(!SoundRequest::default().looping);
    let written: SoundRequest = serde_json::from_str("{}").expect("an empty block");
    assert_eq!(written, SoundRequest::default());
    let looped: SoundRequest = serde_json::from_str(r#"{ "loop": true }"#).expect("a loop");
    assert!(looped.looping);
}

#[test]
fn a_sound_beside_another_block_is_refused() {
    let mut p = sounding(Some(2.0));
    asset_mut(&mut p, "vo-open").speech = Some(SpeechRequest::default());
    assert_only_problem(
        &p,
        E::Sound(S::TwoBlocks {
            asset: asset_id("vo-open"),
            other: F::Speech,
        }),
    );
    let mut p = sounding(Some(2.0));
    asset_mut(&mut p, "vo-open").music = Some(MusicRequest {
        seconds: Some(40),
        ..MusicRequest::default()
    });
    assert_only_problem(
        &p,
        E::Sound(S::TwoBlocks {
            asset: asset_id("vo-open"),
            other: F::Music,
        }),
    );
}

#[test]
fn a_length_the_vendor_will_not_make_is_refused() {
    for found in [MIN_SOUND_SECONDS - 0.1, MAX_SOUND_SECONDS + 0.1, f64::NAN] {
        let errors = problems(&sounding(Some(found)));
        assert!(
            matches!(
                errors.as_slice(),
                [scorsese_core::ValidationError::Asset(E::Sound(
                    S::Length { .. }
                ))]
            ),
            "{found}: {errors:?}"
        );
    }
    for edge in [MIN_SOUND_SECONDS, MAX_SOUND_SECONDS] {
        assert!(sounding(Some(edge)).validate().is_ok(), "{edge} seconds");
    }
}

#[test]
fn a_description_is_not_held_to_a_lines_length() {
    let mut p = sounding(Some(2.0));
    asset_mut(&mut p, "vo-open").prompt = Some("a".repeat(MAX_CHARACTERS + 1));
    assert!(p.validate().is_ok());
}

#[test]
fn a_sound_block_on_another_kind_is_stray() {
    let mut p = project();
    asset_mut(&mut p, "shot-city").sound = Some(SoundRequest::default());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("shot-city"),
            field: F::Sound,
            kind: AssetKind::GeneratedVideo,
        },
    );
}
