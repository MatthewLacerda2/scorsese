//! An imported recording's transcript, read like a line's timings (#1549).

use super::disk::spoken;
use crate::words::{Saying, Word, Words, saying};
use crate::{
    Asset, AssetId, AssetKind, Clip, ClipId, Frames, Project, ProjectPath, Track, TrackId,
    TrackKind,
};

/// An imported recording has words once a transcript sits beside it (#1549),
/// read as they were written, whatever its state says.
#[test]
fn an_imported_recording_with_a_transcript_beside_it_has_words() {
    let dir = crate::scratch::fresh("scorsese-words-imported", "");
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    let sketch = Asset::imported(
        AssetId::new("sketch"),
        AssetKind::Video,
        ProjectPath::new("assets/sketch.mp4"),
    );
    let call = Asset::imported(
        AssetId::new("call"),
        AssetKind::Audio,
        ProjectPath::new("assets/call.wav"),
    );
    assert_eq!(Words::of(&sketch, &dir), None, "not transcribed yet");
    let heard = Words {
        words: vec![Word {
            text: "[risos]".into(),
            start: 0.25,
            end: 0.75,
        }],
    };
    for asset in [&sketch, &call] {
        let beside = Words::file_of(asset).unwrap();
        std::fs::write(
            beside.resolve(&dir).expect("a project path"),
            heard.to_json(),
        )
        .unwrap();
        assert_eq!(Words::of(asset, &dir), Some(heard.clone()), "{}", asset.id);
    }
    assert!(Words::transcribable(AssetKind::Video));
    assert!(Words::transcribable(AssetKind::Audio));
    assert!(!Words::transcribable(AssetKind::GeneratedAudio));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_transcribed_recording_says_its_word_at_an_instant() {
    let (dir, mut project) = spoken("saying-imported", false);
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    let take = ProjectPath::new("assets/take.mov");
    project.assets.push(Asset::imported(
        AssetId::new("take"),
        AssetKind::Video,
        take.clone(),
    ));
    let mut shots = Track::new(TrackId::new("v1"), TrackKind::Video);
    shots.clips.push(Clip::new(
        ClipId::new("shot"),
        AssetId::new("take"),
        Frames(0),
        Frames(90),
    ));
    project.tracks.push(shots);
    let at = |project: &Project| -> Vec<ClipId> {
        saying(project, &dir, 1.75)
            .into_iter()
            .map(|(clip, _)| clip)
            .collect()
    };
    assert_eq!(
        at(&project),
        [ClipId::new("nar")],
        "footage not transcribed"
    );
    let heard = Words {
        words: vec![Word {
            text: "Corta!".into(),
            start: 1.5,
            end: 2.0,
        }],
    };
    std::fs::write(
        Words::transcript(&take)
            .resolve(&dir)
            .expect("a project path"),
        heard.to_json(),
    )
    .unwrap();
    let said = saying(&project, &dir, 1.75);
    let Some((_, Saying::Word(word))) = said.iter().find(|(clip, _)| clip.as_str() == "shot")
    else {
        panic!("{said:?}")
    };
    assert_eq!(word.name, "corta");
    std::fs::remove_dir_all(&dir).ok();
}

/// A camera's film and a recorder's sound of one take often share a name
/// (#1549): each keeps a transcript of its own, named for its whole file name.
#[test]
fn two_recordings_of_one_name_keep_two_transcripts() {
    let dir = crate::scratch::fresh("scorsese-words-pair", "");
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    let film = Asset::imported(
        AssetId::new("film"),
        AssetKind::Video,
        ProjectPath::new("assets/take.mp4"),
    );
    let sound = Asset::imported(
        AssetId::new("sound"),
        AssetKind::Audio,
        ProjectPath::new("assets/take.wav"),
    );
    let transcript = |asset: &Asset| Words::file_of(asset).unwrap();
    assert_eq!(transcript(&film).as_str(), "assets/take.mp4.words.json");
    assert_eq!(transcript(&sound).as_str(), "assets/take.wav.words.json");
    let said = |text: &str| Words {
        words: vec![Word {
            text: text.into(),
            start: 0.0,
            end: 0.5,
        }],
    };
    std::fs::write(
        transcript(&film).resolve(&dir).expect("a project path"),
        said("camera").to_json(),
    )
    .unwrap();
    assert_eq!(
        Words::of(&sound, &dir),
        None,
        "the sound is not transcribed"
    );
    std::fs::write(
        transcript(&sound).resolve(&dir).expect("a project path"),
        said("recorder").to_json(),
    )
    .unwrap();
    assert_eq!(Words::of(&film, &dir), Some(said("camera")));
    assert_eq!(Words::of(&sound, &dir), Some(said("recorder")));

    let mut line = Asset::sketch(AssetId::new("vo"), AssetKind::GeneratedAudio, "a line");
    line.path = Some(ProjectPath::new("generated/vo-abc.mp3"));
    assert_eq!(
        transcript(&line).as_str(),
        "generated/vo-abc.words.json",
        "a line's timings keep their name"
    );
    std::fs::remove_dir_all(&dir).ok();
}
