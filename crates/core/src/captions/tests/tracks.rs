//! A generated track is not a line (#1745): captioning and `saying` take
//! only what was generated as speech.

use super::fixture::{asked, project};
use crate::captions::caption;
use crate::words::saying;
use crate::{
    Asset, AssetId, AssetKind, Clip, ClipId, Frames, GenerationState, MusicRequest, ProjectPath,
    Track, TrackId, TrackKind,
};

/// The fixture's lines, with a generated music bed on an audio track of its
/// own playing under them.
fn with_generated_music(label: &str) -> (std::path::PathBuf, crate::Project) {
    let (dir, mut project) = project(label);
    let mut bed = Asset::sketch(AssetId::new("score"), AssetKind::GeneratedAudio, "a bed");
    bed.music = Some(MusicRequest::default());
    bed.state = Some(GenerationState::Generated);
    bed.path = Some(ProjectPath::new("generated/score-abc.mp3"));
    project.assets.push(bed);
    let mut score = Track::new(TrackId::new("score"), TrackKind::Audio);
    score.clips.push(Clip::new(
        ClipId::new("under"),
        AssetId::new("score"),
        Frames(0),
        Frames(300),
    ));
    project.tracks.push(score);
    (dir, project)
}

#[test]
fn a_generated_track_is_neither_captioned_nor_reported_untimed() {
    let (dir, mut project) = with_generated_music("generated-music");
    let report = caption(&mut project, &dir, &asked()).unwrap();
    assert_eq!(report.lines, [(ClipId::new("nar"), 2)]);
    assert_eq!(report.untimed, [ClipId::new("nar-2")], "the bed is no line");
}

#[test]
fn a_named_track_of_generated_music_has_nothing_to_caption() {
    let (dir, mut project) = with_generated_music("generated-music-named");
    let mut asked = asked();
    asked.narration = vec![TrackId::new("a1"), TrackId::new("score")];
    let report = caption(&mut project, &dir, &asked).unwrap();
    assert_eq!(report.lines, [(ClipId::new("nar"), 2)]);
    assert_eq!(report.untimed, [ClipId::new("nar-2")], "the bed is no line");
}

#[test]
fn saying_lists_the_line_and_not_the_track_under_it() {
    let (dir, project) = with_generated_music("generated-music-saying");
    let said: Vec<ClipId> = saying(&project, &dir, 1.75)
        .into_iter()
        .map(|(clip, _)| clip)
        .collect();
    assert_eq!(said, [ClipId::new("nar")]);
}
