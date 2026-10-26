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
        let beside = Words::beside(asset.path.as_ref().unwrap());
        std::fs::write(beside.resolve(&dir), heard.to_json()).unwrap();
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
    std::fs::write(Words::beside(&take).resolve(&dir), heard.to_json()).unwrap();
    let said = saying(&project, &dir, 1.75);
    let Some((_, Saying::Word(word))) = said.iter().find(|(clip, _)| clip.as_str() == "shot")
    else {
        panic!("{said:?}")
    };
    assert_eq!(word.name, "corta");
    std::fs::remove_dir_all(&dir).ok();
}
