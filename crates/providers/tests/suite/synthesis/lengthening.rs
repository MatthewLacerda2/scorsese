//! A recipe rewritten longer than its last bake (#1062): its clip is lengthened
//! before the bake that will fill it, and a render is told only about what a
//! bake cannot fix.

use crate::common::{project, synth_asset, write};
use scorsese_core::{AssetId, Clip, ClipId, Frames, Project, Track, TrackId, TrackKind};
use scorsese_providers::synth::{bake_asset, out_of_date};

/// Two seconds of music per `a` — four beats at 120 bpm — `times` times over.
fn song(times: usize) -> String {
    let arrangement = vec!["\"a\""; times].join(", ");
    format!(
        r#"{{
  "recipe": "song", "bpm": 120, "seed": 3,
  "tracks": [{{ "name": "pad", "patch": {{ "source": {{ "kind": "karplus" }},
      "amp": {{ "a": 0.001, "d": 0.2, "s": 0.0, "r": 0.1 }} }} }}],
  "patterns": {{ "a": {{ "beats": 4, "notes": [
      {{ "track": "pad", "note": "C4", "start": 0, "dur": 1 }} ] }} }},
  "arrangement": [{arrangement}]
}}"#
    )
}

/// The song baked once at its short length, under a clip of `frames`.
fn baked_short(label: &str, frames: u64) -> (std::path::PathBuf, Project, AssetId) {
    let (dir, mut project) = project(label);
    write(&dir, "recipes/score.json", &song(1));
    let id = synth_asset(&mut project, "score", "recipes/score.json");
    let mut music = Track::new(TrackId::new("music"), TrackKind::Audio);
    let clip = Clip::new(ClipId::new("bed"), id.clone(), Frames(0), Frames(frames));
    music.clips.push(clip);
    project.tracks.push(music);
    bake_asset(&mut project, &dir, &id).expect("bakes");
    (dir, project, id)
}

/// The order `guide recipes` gives — write the recipe, place the clip, bake —
/// with nothing refused in between, though the clip outlasts the last bake.
#[test]
fn a_recipe_written_longer_is_placed_before_it_is_baked() {
    let (dir, mut project, id) = baked_short("lengthen", 30);
    write(&dir, "recipes/score.json", &song(4));
    project.tracks[0].clips[0].duration = Frames(180);
    assert_eq!(project.validate(), Ok(()), "the old bake bounds nothing");
    let told = out_of_date(&project, &dir);
    assert!(
        told.len() == 1 && told[0].contains("older than"),
        "{told:?}"
    );

    bake_asset(&mut project, &dir, &id).expect("bakes the new length");
    assert_eq!(project.validate(), Ok(()));
    assert!(out_of_date(&project, &dir).is_empty(), "{told:?}");
    std::fs::remove_dir_all(dir).ok();
}

/// A bake that is current and still shorter than its clip is what the render
/// says, naming the clip — baking again would only make the same file.
#[test]
fn a_clip_longer_than_a_current_bake_is_told() {
    let (dir, project, _) = baked_short("outlast", 180);
    assert_eq!(project.validate(), Ok(()));
    let told = out_of_date(&project, &dir);
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(
        told[0].contains("clip `bed` plays it to 6.00 s") && told[0].contains("lengthen"),
        "{told:?}"
    );
    std::fs::remove_dir_all(dir).ok();
}
