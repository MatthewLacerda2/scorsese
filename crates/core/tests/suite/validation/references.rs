//! Identity and reference checks: clips point at real assets, on tracks that
//! can hold them.

use crate::common::{assert_only_problem, asset_id, clip_id, project, track_id};
use scorsese_core::{AssetKind, Clip, Frames, TimelineProblem as E, TrackKind};

#[test]
fn a_clip_pointing_at_a_missing_asset_is_reported() {
    let mut p = project();
    p.tracks[0].clips[0].asset = asset_id("shot-citty");
    assert_only_problem(
        &p,
        E::DanglingAssetRef {
            clip: clip_id("c-shot"),
            asset: asset_id("shot-citty"),
        },
    );
}

#[test]
fn a_reused_clip_id_is_reported_across_tracks() {
    let mut p = project();
    p.tracks[1].clips[0].id = clip_id("c-shot");
    assert_only_problem(
        &p,
        E::DuplicateClipId {
            id: clip_id("c-shot"),
        },
    );
}

#[test]
fn a_reused_track_id_is_reported() {
    let mut p = project();
    p.tracks[1].id = track_id("v1");
    assert_only_problem(&p, E::DuplicateTrackId { id: track_id("v1") });
}

#[test]
fn sound_does_not_belong_on_a_video_track() {
    let mut p = project();
    let mut vo = p.tracks[2].clips.remove(0);
    vo.start = Frames(400); // clear of the video clips, so only kind is wrong
    p.tracks[0].clips.push(vo);
    assert_only_problem(
        &p,
        E::TrackKindMismatch {
            track: track_id("v1"),
            track_kind: TrackKind::Video,
            clip: clip_id("c-vo"),
            asset_kind: AssetKind::GeneratedAudio,
        },
    );
}

#[test]
fn picture_does_not_belong_on_an_audio_track() {
    let mut p = project();
    let mut logo = p.tracks[1].clips.remove(0);
    logo.start = Frames(210); // clear of the narration clip
    p.tracks[2].clips.push(logo);
    assert_only_problem(
        &p,
        E::TrackKindMismatch {
            track: track_id("a1"),
            track_kind: TrackKind::Audio,
            clip: clip_id("c-logo"),
            asset_kind: AssetKind::Image,
        },
    );
}

#[test]
fn every_asset_kind_has_a_track_it_can_sit_on() {
    // Otherwise a kind could be added that no timeline can ever hold, and the
    // only symptom would be a confusing mismatch error at the far end.
    let kinds = [
        AssetKind::Video,
        AssetKind::Image,
        AssetKind::Audio,
        AssetKind::Text,
        AssetKind::Color,
        AssetKind::GeneratedVideo,
        AssetKind::GeneratedAudio,
        AssetKind::SynthAudio,
    ];
    for kind in kinds {
        assert!(
            kind.is_visual() || kind.is_audible(),
            "{kind:?} belongs nowhere"
        );
    }
}

#[test]
fn a_clip_built_in_code_passes_the_same_checks() {
    let mut p = project();
    let clip = Clip::new(
        clip_id("c-extra"),
        asset_id("logo"),
        Frames::ZERO,
        Frames(30),
    );
    p.tracks[1].clips.push(clip);
    assert_eq!(p.validate(), Ok(()));
}
