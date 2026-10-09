//! A project to caption: one timed line, one untimed, and music.

use std::path::PathBuf;

use crate::captions::{Captioning, Chunking};
use crate::words::{Word, Words};
use crate::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, GenerationState, Project, ProjectPath,
    PropertyPath, TextStyle, Track, TrackId, TrackKind,
};

/// A folder with a timed line `vo-1` played by `nar` from frame 30 (1 s), an
/// untimed one `vo-2`, and music.
pub(super) fn project(label: &str) -> (PathBuf, Project) {
    let dir =
        std::env::temp_dir().join(format!("scorsese-captions-{label}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("generated")).unwrap();
    let mut project = Project::new("captions", Fps::THIRTY);
    for (id, timed) in [("vo-1", true), ("vo-2", false)] {
        let audio = ProjectPath::new(format!("generated/{id}-abc.mp3"));
        let mut line = Asset::sketch(AssetId::new(id), AssetKind::GeneratedAudio, "a line");
        line.state = Some(GenerationState::Generated);
        line.path = Some(audio.clone());
        if timed {
            // As ElevenLabs returned them for the DataForce ad's first line.
            let words = [
                ("O", 0.52, 0.61),
                ("problema", 0.61, 1.08),
                ("não", 1.08, 1.30),
                ("é", 1.30, 1.38),
                ("falta", 1.38, 1.72),
                ("de", 1.72, 1.80),
                ("dados.", 1.80, 2.31),
                ("É", 2.62, 2.70),
                ("o", 2.70, 2.76),
                ("que", 2.76, 2.90),
                ("fazer", 2.90, 3.20),
                ("com", 3.20, 3.36),
                ("eles.", 3.36, 3.90),
            ];
            let words = Words {
                words: words
                    .into_iter()
                    .map(|(text, start, end)| Word {
                        text: text.into(),
                        start,
                        end,
                    })
                    .collect(),
            };
            std::fs::write(Words::beside(&audio).resolve(&dir), words.to_json()).unwrap();
        }
        project.assets.push(line);
    }
    project.assets.push(Asset::imported(
        AssetId::new("song"),
        AssetKind::Audio,
        ProjectPath::new("assets/song.mp3"),
    ));
    let mut voice = Track::new(TrackId::new("a1"), TrackKind::Audio);
    let at = |id: &str, asset: &str, start, duration| {
        Clip::new(
            ClipId::new(id),
            AssetId::new(asset),
            Frames(start),
            Frames(duration),
        )
    };
    voice.clips.push(at("nar", "vo-1", 30, 120));
    voice.clips.push(at("nar-2", "vo-2", 200, 60));
    let mut music = Track::new(TrackId::new("music"), TrackKind::Audio);
    music.clips.push(at("bed", "song", 0, 300));
    project.tracks.extend([voice, music]);
    (dir, project)
}

pub(super) fn asked() -> Captioning {
    Captioning {
        track: TrackId::new("captions"),
        narration: Vec::new(),
        chunking: Chunking::DEFAULT,
        style: TextStyle::default(),
        lift: 0.2,
        arrive: 8,
        reveal: PropertyPath::new("reveal"),
        height: PropertyPath::new("transform.position.y"),
    }
}
