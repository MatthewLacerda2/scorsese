//! What a generated track asks for (#1264): one block per asset, a length
//! the vendor composes, and nowhere but `generated_audio`.

use crate::common::{assert_only_problem, asset_id, asset_mut, project};
use scorsese_core::{
    AssetField as F, AssetKind, AssetProblem as E, AudioBlock, MAX_CHARACTERS, MAX_MUSIC_SECONDS,
    MIN_MUSIC_SECONDS, MusicProblem as M, MusicRequest, Project, SpeechRequest,
};

/// The fixture's narration asset turned into a track of `seconds`.
fn composing(seconds: Option<u32>) -> Project {
    let mut p = project();
    asset_mut(&mut p, "vo-open").music = Some(MusicRequest {
        seconds,
        ..MusicRequest::default()
    });
    p
}

#[test]
fn a_track_is_a_generated_audio_asset_with_a_music_block() {
    let p = composing(Some(40));
    p.validate().expect("a track validates");
    let asset = p.assets.iter().find(|a| a.id == asset_id("vo-open"));
    assert_eq!(asset.and_then(|a| a.audio_block()), Some(AudioBlock::Music));
    assert!(
        composing(None).validate().is_ok(),
        "a sketch with no length"
    );
}

#[test]
fn a_line_is_one_without() {
    let p = project();
    let asset = p.assets.iter().find(|a| a.id == asset_id("vo-open"));
    assert_eq!(
        asset.and_then(|a| a.audio_block()),
        Some(AudioBlock::Speech)
    );
}

#[test]
fn a_track_is_instrumental_unless_it_says_otherwise() {
    assert!(MusicRequest::default().instrumental);
    let written: MusicRequest = serde_json::from_str("{}").expect("an empty block");
    assert!(written.instrumental);
}

#[test]
fn both_blocks_on_one_asset_are_refused() {
    let mut p = composing(Some(40));
    asset_mut(&mut p, "vo-open").speech = Some(SpeechRequest::default());
    assert_only_problem(
        &p,
        E::Music(M::BothBlocks {
            asset: asset_id("vo-open"),
        }),
    );
}

#[test]
fn a_length_the_vendor_will_not_compose_is_refused() {
    for found in [MIN_MUSIC_SECONDS - 1, MAX_MUSIC_SECONDS + 1] {
        assert_only_problem(
            &composing(Some(found)),
            E::Music(M::Length {
                asset: asset_id("vo-open"),
                found,
                min: MIN_MUSIC_SECONDS,
                max: MAX_MUSIC_SECONDS,
            }),
        );
    }
    for edge in [MIN_MUSIC_SECONDS, MAX_MUSIC_SECONDS] {
        assert!(composing(Some(edge)).validate().is_ok(), "{edge} seconds");
    }
}

#[test]
fn a_description_is_not_held_to_a_lines_length() {
    let mut p = composing(Some(40));
    asset_mut(&mut p, "vo-open").prompt = Some("a".repeat(MAX_CHARACTERS + 1));
    assert!(p.validate().is_ok());
}

#[test]
fn a_music_block_on_another_kind_is_stray() {
    let mut p = project();
    asset_mut(&mut p, "shot-city").music = Some(MusicRequest::default());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("shot-city"),
            field: F::Music,
            kind: AssetKind::GeneratedVideo,
        },
    );
}
