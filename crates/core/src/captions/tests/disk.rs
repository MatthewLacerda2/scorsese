//! Captioning a project from the timings stored beside its narration.

use super::fixture::{asked, project};
use crate::captions::{CaptionError, caption};
use crate::{Asset, AssetId, Clip, ClipId, Frames, Track, TrackId, TrackKind};

#[test]
fn each_caption_arrives_on_its_first_stored_word() {
    let (dir, mut project) = project("timed");
    let report = caption(&mut project, &dir, &asked()).unwrap();
    assert_eq!(report.lines, [(ClipId::new("nar"), 2)]);
    assert_eq!(
        report.untimed,
        [ClipId::new("nar-2")],
        "music is not a line"
    );

    let track = project.tracks.last().unwrap();
    assert_eq!(track.id.as_str(), "captions");
    let at: Vec<(u64, u64)> = track
        .clips
        .iter()
        .map(|clip| (clip.start.get(), clip.end().get()))
        .collect();
    // "O" is said 0.52 s into a line that starts at 1 s: 1.52 − 0.07 lead is
    // 1.45 s, frame 44 (43.5 rounds up). "É" at 3.62 − 0.07 = 3.55 s is frame
    // 107 (106.5), and the first caption stays until it, since the pause
    // between them is short. The second holds 0.5 s past "eles." (4.90 s):
    // 5.40 s, frame 162.
    assert_eq!(at, [(44, 107), (107, 162)]);

    let first = project.asset(&AssetId::new("caption-nar-1")).unwrap();
    assert_eq!(
        first.text.as_deref(),
        Some("O problema não é falta de dados.")
    );
    let reveal = first.text_style().reveal.unwrap();
    assert_eq!(reveal.stagger, 0.0, "the whole caption arrives at once");
    assert!(project.validate().is_ok(), "{:?}", project.validate());
}

#[test]
fn a_rerun_replaces_its_own_captions_and_nothing_else() {
    let (dir, mut project) = project("rerun");
    let mut asked = asked();
    asked.track = TrackId::new("v1");
    let mut titles = Track::new(TrackId::new("v1"), TrackKind::Video);
    titles.clips.push(Clip::new(
        ClipId::new("title"),
        AssetId::new("t"),
        Frames(0),
        Frames(30),
    ));
    project.assets.push(Asset::text(AssetId::new("t"), "Title"));
    project.tracks.push(titles);

    caption(&mut project, &dir, &asked).unwrap();
    // The line moves a second later; running again is all it takes.
    project.tracks[0].clips[0].start = Frames(60);
    let report = caption(&mut project, &dir, &asked).unwrap();
    assert_eq!(report.replaced, 2);
    let track = project
        .tracks
        .iter()
        .find(|t| t.id.as_str() == "v1")
        .unwrap();
    let ids: Vec<&str> = track.clips.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["title", "caption-nar-1", "caption-nar-2"]);
    assert_eq!(track.clips[1].start, Frames(74));
    let captions = project
        .assets
        .iter()
        .filter(|a| a.id.as_str().starts_with("caption-"))
        .count();
    assert_eq!(captions, 2, "no stale caption assets left behind");
}

#[test]
fn a_clip_in_the_way_refuses_the_run_and_changes_nothing() {
    let (dir, mut project) = project("overlap");
    let mut asked = asked();
    asked.track = TrackId::new("v1");
    let mut shots = Track::new(TrackId::new("v1"), TrackKind::Video);
    shots.clips.push(Clip::new(
        ClipId::new("shot"),
        AssetId::new("t"),
        Frames(0),
        Frames(100),
    ));
    project.assets.push(Asset::text(AssetId::new("t"), "x"));
    project.tracks.push(shots);
    let before = project.clone();
    let error = caption(&mut project, &dir, &asked).unwrap_err();
    assert!(matches!(error, CaptionError::Overlap { .. }), "{error}");
    assert_eq!(project, before);

    asked.track = TrackId::new("a1");
    let error = caption(&mut project, &dir, &asked).unwrap_err();
    assert_eq!(error, CaptionError::NotVideo(TrackId::new("a1")));
}
