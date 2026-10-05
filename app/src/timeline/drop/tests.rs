//! The drop's arithmetic and the lane it picks, without a window.

use super::*;
use scorsese_core::{Asset, AssetKind, Clip, ProjectPath, Track, TrackKind};

/// A 4-second shot already on `v1` at 0..120, a title with no length, a
/// sound, and an empty audio track.
fn project() -> Project {
    let mut project = Project::new("t", Fps::THIRTY);
    let mut shot = Asset::imported(
        AssetId::new("shot"),
        AssetKind::Video,
        ProjectPath::new("assets/shot.mp4"),
    );
    shot.media = Some(scorsese_core::MediaMetadata {
        duration_seconds: Some(4.0),
        ..Default::default()
    });
    project.assets.push(shot);
    project
        .assets
        .push(Asset::text(AssetId::new("title"), "hi"));
    project.assets.push(Asset::imported(
        AssetId::new("music"),
        AssetKind::Audio,
        ProjectPath::new("assets/m.wav"),
    ));
    let mut video = Track::new(TrackId::new("v1"), TrackKind::Video);
    video.clips.push(Clip::new(
        ClipId::new("head"),
        AssetId::new("shot"),
        Frames(0),
        Frames(120),
    ));
    project.tracks.push(video);
    project
        .tracks
        .push(Track::new(TrackId::new("a1"), TrackKind::Audio));
    project
}

fn landing(track: &str, start: u64, duration: u64) -> Landing {
    Landing {
        onto: Onto::Track(TrackId::new(track)),
        start: Frames(start),
        duration: Frames(duration),
    }
}

fn anew(start: u64, duration: u64) -> Landing {
    Landing {
        onto: Onto::NewTrack,
        ..landing("", start, duration)
    }
}

/// A measured shot is placed whole; a title, which has no length of its
/// own, gets five seconds rather than a refusal.
#[test]
fn a_placed_clip_runs_the_whole_asset_or_five_seconds() {
    let project = project();
    assert_eq!(duration_of(&project, &AssetId::new("shot")), Frames(120));
    assert_eq!(duration_of(&project, &AssetId::new("title")), Frames(150));
}

/// Dropped past the end of what is there, the clip lands, gets an id of
/// its own, and is the document's rather than a copy of it.
#[test]
fn a_drop_on_a_free_stretch_places_a_clip() {
    let before = project();
    let (after, clip, _) = place(&before, &AssetId::new("shot"), &landing("v1", 120, 120))
        .expect("the stretch is free");
    assert_eq!(clip, ClipId::new("shot"));
    assert_eq!(after.tracks[0].clips.len(), 2);
    assert_eq!(
        before.tracks[0].clips.len(),
        1,
        "the window's document is untouched until saved"
    );
}

/// The refusal a hand will actually meet: landing on a clip already there.
#[test]
fn a_drop_on_a_clip_already_there_is_refused_with_a_reason() {
    let project = project();
    let overlap = place(&project, &AssetId::new("title"), &landing("v1", 60, 150));
    assert!(
        overlap
            .expect_err("frames 60-120 are taken")
            .contains("head")
    );
}

/// A sound over a picture lane, or below the last lane, goes on a new
/// lane; a sound over the sound lane goes on it; the gap between two
/// lanes is nowhere.
#[test]
fn a_lane_of_the_other_kind_or_none_at_all_makes_a_new_one() {
    let project = project();
    let music = AssetId::new("music");
    let (video, audio) = (&project.tracks[0], &project.tracks[1]);
    assert_eq!(
        onto(&project, &music, Some(video), false),
        Some(Onto::NewTrack)
    );
    assert_eq!(
        onto(&project, &music, Some(audio), false),
        Some(Onto::Track(audio.id.clone()))
    );
    assert_eq!(onto(&project, &music, None, true), Some(Onto::NewTrack));
    assert_eq!(onto(&project, &music, None, false), None);
}

/// The new lane is the asset's kind and holds the clip, at the frame it
/// was dropped at — on an empty timeline too.
#[test]
fn a_new_lane_is_made_with_the_clip_on_it() {
    let (after, clip, track) = place(&project(), &AssetId::new("music"), &anew(300, 30))
        .expect("a new audio lane is empty");
    let lane = after.tracks.iter().find(|lane| lane.id == track);
    let lane = lane.expect("the lane is there");
    assert_eq!((track.as_str(), lane.kind), ("a2", TrackKind::Audio));
    assert_eq!(
        (lane.clips[0].id.clone(), lane.clips[0].start),
        (clip, Frames(300))
    );

    let mut empty = project();
    empty.tracks.clear();
    let (after, _, track) =
        place(&empty, &AssetId::new("title"), &anew(0, 150)).expect("the first lane is made");
    assert_eq!((track.as_str(), after.tracks.len()), ("v1", 1));
    assert_eq!(after.tracks[0].kind, TrackKind::Video);
}

/// Dropped a few frames short of the last clip's end, a shot is pulled
/// onto it rather than leaving a gap that renders as a black flash.
#[test]
fn a_drop_near_a_cut_is_pulled_onto_it() {
    let project = project();
    let start = snapped(&project, Frames(900), Frames(124), Frames(120), Frames(8));
    assert_eq!(start, Frames(120));
    let far = snapped(&project, Frames(900), Frames(400), Frames(120), Frames(8));
    assert_eq!(far, Frames(400));
}
