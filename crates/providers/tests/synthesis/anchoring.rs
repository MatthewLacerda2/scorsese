//! A song's sections anchored at clips (#1009): each lands where its clip
//! starts, moves when the clip does, and is refused by name when it cannot.

use crate::common::{project, synth_asset, write};
use scorsese_core::{AssetId, Clip, ClipId, Frames, Project, Track, TrackId, TrackKind};
use scorsese_providers::synth::{SynthesisError, bake_asset, out_of_date};

/// Four sections of two seconds each as written, the third anchored at the
/// logo and the fourth at the call to action, ending on the last beat.
const SONG: &str = r#"{
  "recipe": "song", "bpm": 120, "seed": 3,
  "tracks": [{ "name": "pad", "patch": { "source": { "kind": "karplus" },
      "amp": { "a": 0.001, "d": 0.2, "s": 0.0, "r": 0.1 } } }],
  "patterns": { "a": { "beats": 4, "notes": [
      { "track": "pad", "note": "C4", "start": 0, "dur": 1 } ] } },
  "arrangement": ["a", "a", "a", "a"],
  "anchors": [{ "section": 2, "clip": "logo" }, { "section": 3, "clip": "cta" }],
  "tail": "exact"
}"#;

/// The song playing from one second in, under a picture track whose logo
/// and call to action start `logo` and `cta` frames (30 fps) after it.
fn scored(label: &str, logo: u64, cta: u64) -> (std::path::PathBuf, Project, AssetId) {
    let (dir, mut project) = project(label);
    write(&dir, "recipes/score.json", SONG);
    let id = synth_asset(&mut project, "score", "recipes/score.json");
    let mut music = Track::new(TrackId::new("music"), TrackKind::Audio);
    music.clips.push(clip("bed", &id, 30, 300));
    let mut picture = Track::new(TrackId::new("picture"), TrackKind::Video);
    let shot = AssetId::new("shot");
    picture.clips.push(clip("logo", &shot, 30 + logo, 30));
    picture.clips.push(clip("cta", &shot, 30 + cta, 30));
    project.tracks.extend([music, picture]);
    (dir, project, id)
}

fn clip(id: &str, asset: &AssetId, start: u64, frames: u64) -> Clip {
    Clip::new(
        ClipId::new(id),
        asset.clone(),
        Frames(start),
        Frames(frames),
    )
}

fn seconds(project: &Project, id: &AssetId) -> f64 {
    let media = project.asset(id).and_then(|asset| asset.media.as_ref());
    media
        .and_then(|it| it.duration_seconds)
        .expect("a baked length")
}

/// Logo 4.4 s and CTA 6.2 s into the song: the last section, after both,
/// plays its written two seconds — so the song ends at 8.2. Move the CTA and
/// the bake is out of date until it is baked again.
#[test]
fn sections_land_on_their_clips_and_follow_them() {
    let (dir, mut project, id) = scored("anchor-clips", 132, 186);
    let first = bake_asset(&mut project, &dir, &id).expect("bakes");
    assert!((seconds(&project, &id) - 8.2).abs() < 1e-3);

    project.tracks[1].clips[1].start = Frames(30 + 183);
    assert_eq!(
        out_of_date(&project, &dir).len(),
        1,
        "a moved anchor is stale"
    );
    let second = bake_asset(&mut project, &dir, &id).expect("re-bakes");
    assert_ne!(first.path(), second.path());
    assert!((seconds(&project, &id) - 8.1).abs() < 1e-3);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_anchor_the_project_cannot_place_is_refused_by_name() {
    let (dir, mut project, id) = scored("anchor-refused", 132, 186);
    project.tracks[1].clips[0].id = ClipId::new("elsewhere");
    let said = bake_asset(&mut project, &dir, &id).expect_err("no logo");
    assert!(
        said.to_string().contains("clip `logo` names no clip"),
        "{said}"
    );

    project.tracks[1].clips[0].id = ClipId::new("logo");
    project.tracks[1].clips[0].start = Frames(0);
    let said = bake_asset(&mut project, &dir, &id).expect_err("before the music");
    assert!(
        said.to_string().contains("1.000 s before the song"),
        "{said}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The CTA moved ahead of the logo: both clips named, since those are the
/// two to move.
#[test]
fn clips_the_wrong_way_round_are_both_named() {
    let (dir, mut project, id) = scored("anchor-backwards", 186, 132);
    let refused = bake_asset(&mut project, &dir, &id).expect_err("backwards");
    let said = refused.to_string();
    assert!(
        matches!(refused, SynthesisError::AnchorsOutOfOrder { .. }),
        "{said}"
    );
    assert!(said.contains("`cta`") && said.contains("`logo`"), "{said}");
    std::fs::remove_dir_all(dir).ok();
}
