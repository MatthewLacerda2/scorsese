//! Reading a line's timings off disk, and what is said at an instant.

use crate::words::{Saying, Word, Words, saying};
use crate::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, GenerationState, Project, ProjectPath,
    Track, TrackId, TrackKind,
};

/// A project folder with one generated line `vo`, timed when `timed`, played
/// by clip `nar` from frame 30 (1 s) — and its title, which is no narration.
fn spoken(label: &str, timed: bool) -> (std::path::PathBuf, Project) {
    let dir = std::env::temp_dir().join(format!("scorsese-words-{label}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("generated")).unwrap();
    let audio = ProjectPath::new("generated.d/vo-abc.mp3");
    let mut line = Asset::sketch(AssetId::new("vo"), AssetKind::GeneratedAudio, "a line");
    line.state = Some(GenerationState::Generated);
    line.path = Some(audio.clone());
    if timed {
        let words = Words {
            words: vec![Word {
                text: "Line.".into(),
                start: 0.5,
                end: 1.0,
            }],
        };
        std::fs::create_dir_all(dir.join("generated.d")).unwrap();
        std::fs::write(Words::beside(&audio).resolve(&dir), words.to_json()).unwrap();
    }
    let mut project = Project::new("words", Fps::THIRTY);
    project.assets.push(line);
    project
        .assets
        .push(Asset::sketch(AssetId::new("t"), AssetKind::Text, "x"));
    let mut track = Track::new(TrackId::new("a1"), TrackKind::Audio);
    track.clips.push(Clip::new(
        ClipId::new("nar"),
        AssetId::new("vo"),
        Frames(30),
        Frames(60),
    ));
    track.clips.push(Clip::new(
        ClipId::new("title"),
        AssetId::new("t"),
        Frames(0),
        Frames(90),
    ));
    project.tracks.push(track);
    (dir, project)
}

#[test]
fn only_a_generated_line_with_a_file_beside_it_has_words() {
    let (dir, mut project) = spoken("of", true);
    assert_eq!(
        Words::beside(&ProjectPath::new("generated.d/vo")).as_str(),
        "generated.d/vo.words.json",
        "a dot in a folder is not an extension"
    );
    let line = project.assets[0].clone();
    assert_eq!(Words::of(&line, &dir).map(|w| w.words.len()), Some(1));
    project.assets[0].state = Some(GenerationState::Stale);
    assert_eq!(Words::of(&project.assets[0], &dir), None, "stale");
    let imported = Asset {
        kind: AssetKind::Audio,
        ..line
    };
    assert_eq!(Words::of(&imported, &dir), None, "imported audio");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_word_being_said_is_named_and_an_untimed_line_says_so() {
    let (dir, project) = spoken("saying", true);
    let at = |seconds| saying(&project, &dir, seconds);
    let Saying::Word(word) = &at(1.75)[0].1 else {
        panic!("{:?}", at(1.75))
    };
    assert_eq!((word.name.as_str(), word.start), ("line", 1.5));
    let between = [(ClipId::new("nar"), Saying::Between)];
    assert_eq!(at(1.25), between);
    assert_eq!(at(2.0), between, "a word is over at its end");
    assert_eq!(at(2.5), between, "the line plays on after its last word");
    assert_eq!(at(0.5), [], "before the line plays");
    assert_eq!(at(3.0), [], "after it");
    std::fs::remove_dir_all(&dir).ok();
    let (dir, project) = spoken("untimed", false);
    assert_eq!(saying(&project, &dir, 1.75)[0].1, Saying::Untimed);
    std::fs::remove_dir_all(&dir).ok();
}
