//! A song fitted `to: "clip"` (#1000): the length comes from the timeline,
//! moves with it, and a render says so when the bake has not caught up.

use crate::common::{project, synth_asset, write};
use scorsese_core::{AssetId, Clip, ClipId, Frames, Project, Track, TrackId, TrackKind};
use scorsese_providers::synth::{SynthesisError, bake_asset, out_of_date};

/// Two seconds of music as written — four beats at 120 bpm — fitted to
/// whatever clip plays it.
const SONG: &str = r#"{
  "recipe": "song", "bpm": 120, "seed": 3,
  "tracks": [{ "name": "pad", "patch": { "source": { "kind": "karplus" },
      "amp": { "a": 0.001, "d": 0.2, "s": 0.0, "r": 0.1 } } }],
  "patterns": { "a": { "beats": 4, "notes": [
      { "track": "pad", "note": "C4", "start": 0, "dur": 3.5 } ] } },
  "arrangement": ["a"],
  "fit": { "to": "clip", "mode": "loop" }
}"#;

/// A project with the song in it and an audio track to place it on.
fn with_song(label: &str) -> (std::path::PathBuf, Project, AssetId) {
    let (dir, mut project) = project(label);
    write(&dir, "recipes/score.json", SONG);
    let id = synth_asset(&mut project, "score", "recipes/score.json");
    project
        .tracks
        .push(Track::new(TrackId::new("music"), TrackKind::Audio));
    (dir, project, id)
}

/// Places the song as clip `clip`, `frames` long on the 30 fps timeline.
fn place(project: &mut Project, id: &AssetId, clip: &str, start: u64, frames: u64) {
    let clip = Clip::new(ClipId::new(clip), id.clone(), Frames(start), Frames(frames));
    project.tracks[0].clips.push(clip);
}

fn seconds(project: &Project, id: &AssetId) -> f64 {
    let media = project.asset(id).and_then(|asset| asset.media.as_ref());
    media
        .and_then(|media| media.duration_seconds)
        .expect("a baked length")
}

#[test]
fn the_song_is_as_long_as_the_clip_that_plays_it() {
    let (dir, mut project, id) = with_song("fit-clip");
    place(&mut project, &id, "bed", 0, 75);
    bake_asset(&mut project, &dir, &id).expect("bakes");
    assert!(
        (seconds(&project, &id) - 2.5).abs() < 1e-6,
        "75 frames at 30 fps"
    );

    // The in-point counts: a clip starting half a second into its music still
    // needs the last note on its own last frame.
    project.tracks[0].clips[0].source_in = Frames(15);
    bake_asset(&mut project, &dir, &id).expect("bakes again");
    assert!((seconds(&project, &id) - 3.0).abs() < 1e-6);
    std::fs::remove_dir_all(dir).ok();
}

/// The cut changes, the bake's address changes with it — and until it is
/// baked again, a render is told rather than left to play the old length.
#[test]
fn a_new_cut_is_a_new_bake_and_until_then_a_render_is_told() {
    let (dir, mut project, id) = with_song("fit-recut");
    place(&mut project, &id, "bed", 0, 60);
    let first = bake_asset(&mut project, &dir, &id).expect("bakes");
    assert!(out_of_date(&project, &dir).is_empty(), "just baked");

    project.tracks[0].clips[0].duration = Frames(90);
    let told = out_of_date(&project, &dir);
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(
        told[0].contains("`score`") && told[0].contains("synth bake"),
        "{told:?}"
    );

    let second = bake_asset(&mut project, &dir, &id).expect("re-bakes");
    assert!(second.is_fresh(), "a changed length is a cache miss");
    assert_ne!(first.path(), second.path());
    assert!(out_of_date(&project, &dir).is_empty(), "caught up");
    assert!((seconds(&project, &id) - 3.0).abs() < 1e-6);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_song_nothing_plays_has_no_length_to_fit() {
    let (dir, mut project, id) = with_song("fit-unplaced");
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    assert!(
        matches!(refused, SynthesisError::NotPlaced { .. }),
        "{refused}"
    );
    assert!(refused.to_string().contains("fit.seconds"), "{refused}");
    std::fs::remove_dir_all(dir).ok();
}

/// Two clips of one length agree; two of different lengths are refused with
/// both named, rather than fitted to whichever came first.
#[test]
fn clips_of_different_lengths_are_named_and_refused() {
    let (dir, mut project, id) = with_song("fit-twice");
    place(&mut project, &id, "intro", 0, 60);
    place(&mut project, &id, "outro", 300, 60);
    bake_asset(&mut project, &dir, &id).expect("one length between them");

    project.tracks[0].clips[1].duration = Frames(45);
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    let said = refused.to_string();
    assert!(
        matches!(refused, SynthesisError::ClipsDisagree { .. }),
        "{said}"
    );
    assert!(
        said.contains("`intro` 2.00 s") && said.contains("`outro` 1.50 s"),
        "{said}"
    );
    std::fs::remove_dir_all(dir).ok();
}
