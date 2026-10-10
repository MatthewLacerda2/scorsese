//! Cutting a narrated video to its voice.

mod layout;
mod refusals;

use std::path::PathBuf;

use crate::voice::{Scene, Voicing};
use crate::words::{Word, Words};
use crate::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, GenerationState, Project, ProjectPath,
    Track, TrackId, TrackKind,
};

/// A 30 fps folder with three scenes, cut long:
///
/// - `a1`: `nar-1` 0–90 (last word ends 1.5 s in), `nar-2` 100–190 (2.0 s in)
///   and `nar-3` 200–260, imported speech with no word timings;
/// - `v1`: stills `page-1` 0–100, `page-2` 100–200, `page-3` 200–260;
/// - `sfx`: `whoosh` 105–115, scene 2's rider;
/// - `music`: `bed` 0–300, named by nobody.
pub(super) fn project(label: &str) -> (PathBuf, Project) {
    let dir = std::env::temp_dir().join(format!("scorsese-voice-{label}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("generated")).unwrap();
    let mut project = Project::new("voice", Fps::THIRTY);
    for (id, last) in [("vo-1", 1.5), ("vo-2", 2.0)] {
        let audio = ProjectPath::new(format!("generated/{id}-abc.mp3"));
        let mut line = Asset::sketch(AssetId::new(id), AssetKind::GeneratedAudio, "a line");
        line.state = Some(GenerationState::Generated);
        line.path = Some(audio.clone());
        let words = Words {
            words: vec![word("A", 0.1, 0.3), word("line.", 0.4, last)],
        };
        std::fs::write(Words::beside(&audio).resolve(&dir), words.to_json()).unwrap();
        project.assets.push(line);
    }
    for (id, kind, path) in [
        ("vo-3", AssetKind::Audio, "assets/vo-3.wav"),
        ("still", AssetKind::Image, "assets/still.png"),
        ("swoosh", AssetKind::Audio, "assets/swoosh.wav"),
        ("song", AssetKind::Audio, "assets/song.mp3"),
    ] {
        project.assets.push(Asset::imported(
            AssetId::new(id),
            kind,
            ProjectPath::new(path),
        ));
    }
    project.tracks = vec![
        track(
            "v1",
            TrackKind::Video,
            &[
                ("page-1", "still", 0, 100),
                ("page-2", "still", 100, 100),
                ("page-3", "still", 200, 60),
            ],
        ),
        track(
            "a1",
            TrackKind::Audio,
            &[
                ("nar-1", "vo-1", 0, 90),
                ("nar-2", "vo-2", 100, 90),
                ("nar-3", "vo-3", 200, 60),
            ],
        ),
        track("sfx", TrackKind::Audio, &[("whoosh", "swoosh", 105, 10)]),
        track("music", TrackKind::Audio, &[("bed", "song", 0, 300)]),
    ];
    assert!(project.validate().is_ok(), "{:?}", project.validate());
    (dir, project)
}

/// The three scenes in order, `whoosh` riding the second; a 0.2 s gap and
/// nothing else.
pub(super) fn voicing() -> Voicing {
    let scene = |n: u8, riders: &[&str]| Scene {
        line: ClipId::new(format!("nar-{n}")),
        visuals: vec![ClipId::new(format!("page-{n}"))],
        riders: riders.iter().map(|id| ClipId::new(*id)).collect(),
        lead_in: None,
    };
    Voicing {
        scenes: vec![scene(1, &[]), scene(2, &["whoosh"]), scene(3, &[])],
        lead_in: 0.0,
        gap: 0.2,
        overlap: 0.0,
        from: None,
    }
}

/// Where a clip is: its track, start and end.
pub(super) fn at(project: &Project, id: &str) -> (String, u64, u64) {
    let (track, clip) = project
        .clips()
        .find(|(_, clip)| clip.id.as_str() == id)
        .unwrap();
    (track.id.to_string(), clip.start.get(), clip.end().get())
}

fn word(text: &str, start: f64, end: f64) -> Word {
    Word {
        text: text.into(),
        start,
        end,
    }
}

fn track(id: &str, kind: TrackKind, clips: &[(&str, &str, u64, u64)]) -> Track {
    let mut track = Track::new(TrackId::new(id), kind);
    for (clip, asset, start, duration) in clips {
        track.clips.push(Clip::new(
            ClipId::new(*clip),
            AssetId::new(*asset),
            Frames(*start),
            Frames(*duration),
        ));
    }
    track
}
