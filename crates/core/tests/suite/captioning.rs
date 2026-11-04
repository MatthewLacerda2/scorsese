//! A generated track is not a line (#1745): captioning and `saying` take
//! only what was generated as speech.

use std::path::PathBuf;

use scorsese_core::captions::{Captioning, Chunking, Look, caption};
use scorsese_core::words::{Word, Words, saying};
use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, GenerationState, MusicRequest, Project,
    ProjectPath, PropertyPath, TextStyle, Track, TrackId, TrackKind,
};

use crate::common::temp_project_dir;

/// A generated asset `id` realised at `generated/<id>.mp3`.
fn generated(id: &str) -> Asset {
    let mut asset = Asset::sketch(AssetId::new(id), AssetKind::GeneratedAudio, id);
    asset.state = Some(GenerationState::Generated);
    asset.path = Some(ProjectPath::new(format!("generated/{id}.mp3")));
    asset
}

fn clip(id: &str, asset: &str, start: u64, duration: u64) -> Clip {
    Clip::new(
        ClipId::new(id),
        AssetId::new(asset),
        Frames(start),
        Frames(duration),
    )
}

/// A timed line `nar` and an untimed one `nar-2` on `a1`, and a generated
/// music bed `under` on a track of its own playing under both.
fn project(label: &str) -> (PathBuf, Project) {
    let dir = temp_project_dir(label);
    std::fs::create_dir_all(dir.join("generated")).expect("create generated/");
    let mut project = Project::new("captions", Fps::THIRTY);
    let line = generated("vo-1");
    let words = Words {
        words: [("Hello", 0.5, 0.9), ("there.", 0.9, 1.4)]
            .into_iter()
            .map(|(text, start, end)| Word {
                text: text.into(),
                start,
                end,
            })
            .collect(),
    };
    let beside = Words::beside(line.path.as_ref().expect("a generated line has a path"));
    std::fs::write(
        beside.resolve(&dir).expect("a project path"),
        words.to_json(),
    )
    .expect("write the timings");
    let mut bed = generated("score");
    bed.music = Some(MusicRequest::default());
    project.assets.extend([line, generated("vo-2"), bed]);
    let mut voice = Track::new(TrackId::new("a1"), TrackKind::Audio);
    voice
        .clips
        .extend([clip("nar", "vo-1", 30, 90), clip("nar-2", "vo-2", 200, 60)]);
    let mut score = Track::new(TrackId::new("score"), TrackKind::Audio);
    score.clips.push(clip("under", "score", 0, 300));
    project.tracks.extend([voice, score]);
    (dir, project)
}

fn asked(narration: &[&str]) -> Captioning {
    Captioning {
        track: TrackId::new("captions"),
        narration: narration.iter().map(|id| TrackId::new(*id)).collect(),
        chunking: Chunking::DEFAULT,
        look: Look::Text,
        style: TextStyle::default(),
        lift: 0.2,
        arrive: 8,
        reveal: PropertyPath::new("reveal"),
        height: PropertyPath::new("transform.position.y"),
        volume: None,
    }
}

#[test]
fn a_generated_track_is_neither_captioned_nor_reported_untimed() {
    for named in [&[][..], &["a1", "score"][..]] {
        let (dir, mut project) = project("generated-music");
        let report = caption(&mut project, &dir, &asked(named)).expect("captioned");
        assert_eq!(report.lines, [(ClipId::new("nar"), 1)], "{named:?}");
        assert_eq!(report.untimed, [ClipId::new("nar-2")], "{named:?}");
    }
}

#[test]
fn saying_lists_the_line_and_not_the_track_under_it() {
    let (dir, project) = project("generated-music-saying");
    let said: Vec<ClipId> = saying(&project, &dir, 1.6)
        .into_iter()
        .map(|(clip, _)| clip)
        .collect();
    assert_eq!(said, [ClipId::new("nar")]);
}
