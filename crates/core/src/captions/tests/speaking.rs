//! Which transcribed clips a run collects (#1549): only what can speak, and
//! nothing silenced throughout.

use super::fixture::{asked, project};
use crate::captions::caption;
use crate::words::{Word, Words};
use crate::{
    Asset, AssetId, AssetKind, Clip, ClipId, Easing, Frames, Keyframe, KeyframeTrack, ProjectPath,
    PropertyPath, Track, TrackId, TrackKind,
};

/// A transcribed take on a track the caller named, beside a title, a photo
/// and a page: only what can carry speech is collected (#1549), so the rest
/// is not reported as "not transcribed yet".
#[test]
fn a_named_track_collects_only_what_can_speak() {
    let (dir, mut project) = project("footage-named");
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    let take = ProjectPath::new("assets/take.mp4");
    std::fs::write(Words::transcript(&take).resolve(&dir), said().to_json()).unwrap();
    project.assets.push(Asset::imported(
        AssetId::new("take"),
        AssetKind::Video,
        take,
    ));
    project
        .assets
        .push(Asset::sketch(AssetId::new("title"), AssetKind::Text, "x"));
    project.assets.push(Asset::imported(
        AssetId::new("photo"),
        AssetKind::Image,
        ProjectPath::new("assets/photo.png"),
    ));
    project.assets.push(Asset::imported(
        AssetId::new("page"),
        AssetKind::Html,
        ProjectPath::new("pages/card.html"),
    ));
    let mut shots = Track::new(TrackId::new("v1"), TrackKind::Video);
    for (at, (clip, asset)) in [
        ("shot", "take"),
        ("name", "title"),
        ("still", "photo"),
        ("card", "page"),
    ]
    .into_iter()
    .enumerate()
    {
        shots.clips.push(Clip::new(
            ClipId::new(clip),
            AssetId::new(asset),
            Frames(at as u64 * 60),
            Frames(60),
        ));
    }
    project.tracks.insert(0, shots);
    let mut asked = asked();
    asked.narration = vec![TrackId::new("v1")];
    let report = caption(&mut project, &dir, &asked).unwrap();
    assert_eq!(report.lines, [(ClipId::new("shot"), 1)]);
    assert_eq!(report.untimed, Vec::<ClipId>::new());
}

/// A transcribed take whose sound is held at nothing throughout — the
/// picture kept, a narration over it — says nothing on screen either, when no
/// track is named (#1549).
#[test]
fn a_take_silenced_throughout_is_not_captioned() {
    let (dir, mut project) = project("footage-silenced");
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    let take = ProjectPath::new("assets/take.mp4");
    std::fs::write(Words::transcript(&take).resolve(&dir), said().to_json()).unwrap();
    project.assets.push(Asset::imported(
        AssetId::new("take"),
        AssetKind::Video,
        take,
    ));
    let mut shots = Track::new(TrackId::new("v1"), TrackKind::Video);
    for (id, start) in [("silenced", 0), ("heard", 60)] {
        shots.clips.push(Clip::new(
            ClipId::new(id),
            AssetId::new("take"),
            Frames(start),
            Frames(60),
        ));
    }
    let volume = |values: &[f64]| {
        let points = values.iter().enumerate().map(|(at, &value)| Keyframe {
            t: Frames(at as u64 * 30),
            value,
            easing: Easing::default(),
        });
        KeyframeTrack::new(PropertyPath::new("volume"), points.collect())
    };
    shots.clips[0].keyframes = vec![volume(&[1.0]), volume(&[0.0, 0.0])];
    shots.clips[1].keyframes = vec![volume(&[0.0, 1.0])];
    project.tracks.insert(0, shots);
    let mut asked = asked();
    asked.volume = Some(PropertyPath::new("volume"));
    let report = caption(&mut project, &dir, &asked).unwrap();
    assert_eq!(
        report.lines,
        [(ClipId::new("heard"), 1), (ClipId::new("nar"), 2)]
    );
}

fn said() -> Words {
    Words {
        words: vec![Word {
            text: "Corta!".into(),
            start: 0.2,
            end: 0.6,
        }],
    }
}
