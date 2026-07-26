//! Finding out what the project never recorded about its media.
//!
//! Needs ffmpeg and ffprobe on PATH, like everything else that touches real
//! media here. They do not skip themselves when it is absent.

mod common;

use std::borrow::Cow;

use scorsese_core::{AssetKind, Project};
use scorsese_render::{Note, fill_media};

use common::ffmpeg::{fixture_dir, generate, tools};
use common::{audio_track, clip, file_asset, project, silent_asset, video_track};

/// A project with one video clip showing `id`, and a real file behind it.
fn shot(dir: &std::path::Path, tools: &scorsese_render::Tools, sound: bool) -> Project {
    let mut media = vec!["-f", "lavfi", "-i", "color=c=black:s=32x32:d=1:r=30"];
    if sound {
        media.extend([
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-shortest",
        ]);
    }
    generate(tools, &dir.join("assets").join("shot.mp4"), &media);
    project(
        vec![file_asset("shot", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "shot", 0, 30)])],
    )
}

#[test]
fn an_unprobed_video_asset_is_probed_and_what_it_has_recorded() {
    let tools = tools();
    let dir = fixture_dir("fill-sound");
    let project = shot(&dir, &tools, true);

    let (filled, notes) = fill_media(&tools, &project, &dir);

    let asset = filled
        .asset(&scorsese_core::AssetId::new("shot"))
        .expect("asset");
    assert_eq!(
        asset.media.and_then(|media| media.audio_channels),
        Some(1),
        "the sound on the file is what decides it is mixed, so it has to be found"
    );
    assert!(
        notes.is_empty(),
        "a probe that worked is not news: {notes:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_project_that_already_knows_is_left_exactly_as_it_was() {
    // No clone and no subprocess. The assets table is meant to hold this, and a
    // render that re-learns it every time is paying for the table twice.
    let tools = tools();
    let dir = fixture_dir("fill-known");
    let project = project(
        vec![silent_asset("shot", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "shot", 0, 30)])],
    );

    let (filled, notes) = fill_media(&tools, &project, &dir);

    assert!(matches!(filled, Cow::Borrowed(_)), "nothing to find out");
    assert!(notes.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_audio_assets_metadata_is_nobodys_business_here() {
    // A clip on an audio track is mixed whatever its metadata says — being
    // there is already the answer to the question this pass asks — so probing
    // it would be a process spawned to learn nothing.
    let tools = tools();
    let dir = fixture_dir("fill-audio");
    generate(
        &tools,
        &dir.join("assets").join("music.wav"),
        &["-f", "lavfi", "-i", "sine=frequency=440:duration=1"],
    );
    let project = project(
        vec![
            silent_asset("shot", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "shot", 0, 30)]),
            audio_track("a1", vec![clip("m1", "music", 0, 30)]),
        ],
    );

    let (filled, _) = fill_media(&tools, &project, &dir);

    assert!(matches!(filled, Cow::Borrowed(_)), "nothing worth probing");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_file_that_cannot_be_probed_leaves_a_note_naming_the_clip() {
    // The failure this whole pass exists to prevent: a clip whose sound was
    // left out, and nothing said about it. Whatever else happens, that never
    // happens quietly.
    let tools = tools();
    let dir = fixture_dir("fill-broken");
    std::fs::create_dir_all(dir.join("assets")).expect("create assets dir");
    std::fs::write(dir.join("assets").join("shot.mp4"), b"not a video at all")
        .expect("write the broken file");
    let project = project(
        vec![file_asset("shot", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "shot", 0, 30)])],
    );

    let (filled, notes) = fill_media(&tools, &project, &dir);

    let asset = filled
        .asset(&scorsese_core::AssetId::new("shot"))
        .expect("asset");
    assert!(asset.media.is_none(), "a failed probe invents nothing");
    match notes.as_slice() {
        [Note::ClipAudioSkipped { clip, reason }] => {
            assert_eq!(clip, "c1");
            assert!(!reason.is_empty(), "the note has to say why");
        }
        other => panic!("expected one skipped-audio note, got {other:?}"),
    }
    std::fs::remove_dir_all(&dir).ok();
}
