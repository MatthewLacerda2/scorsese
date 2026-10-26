//! Transcribing an asset: what is refused, what is kept, and where the words
//! are written.

use std::cell::Cell;
use std::path::Path;

use scorsese_core::words::{Word, Words};
use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project, ProjectPath};
use scorsese_providers::transcribe::{
    Asked, Heard, Language, TranscribeError, Transcriber, transcribe,
};

/// A Whisper that hears one word, and counts how often it was asked.
#[derive(Default)]
struct Ear {
    asked: Cell<usize>,
}

impl Transcriber for Ear {
    fn transcribe(
        &self,
        wav: Vec<u8>,
        language: Option<&Language>,
    ) -> Result<Heard, TranscribeError> {
        self.asked.set(self.asked.get() + 1);
        assert_eq!(wav, b"WAV of assets/take.mp4");
        Ok(Heard {
            language: language.map_or("pt", Language::as_str).to_owned(),
            duration: 2.0,
            words: Words {
                words: vec![Word {
                    text: "Corta!".into(),
                    start: 0.5,
                    end: 0.9,
                }],
            },
        })
    }
}

/// A folder with footage `take`, a picture, a generated line and footage whose
/// file is gone.
fn folder(label: &str) -> (std::path::PathBuf, Project) {
    let dir = std::env::temp_dir().join(format!(
        "scorsese-transcribe-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("assets")).expect("as set up");
    std::fs::write(dir.join("assets/take.mp4"), b"footage").expect("as set up");
    let mut project = Project::new("t", Fps::THIRTY);
    let imported = |id: &str, kind, path: &str| {
        Asset::imported(AssetId::new(id), kind, ProjectPath::new(path))
    };
    project.assets.extend([
        imported("take", AssetKind::Video, "assets/take.mp4"),
        imported("still", AssetKind::Image, "assets/still.png"),
        imported("gone", AssetKind::Audio, "assets/gone.wav"),
        Asset::sketch(AssetId::new("vo"), AssetKind::GeneratedAudio, "a line"),
    ]);
    (dir, project)
}

fn decode(file: &Path) -> Result<Vec<u8>, String> {
    let name = file
        .strip_prefix(
            file.parent()
                .expect("as set up")
                .parent()
                .expect("as set up"),
        )
        .expect("as set up");
    Ok(format!("WAV of {}", name.display()).into_bytes())
}

fn asked(again: bool) -> Asked {
    Asked {
        language: None,
        again,
    }
}

#[test]
fn the_words_are_written_beside_the_footage_and_kept_until_asked_again() {
    let (dir, project) = folder("writes");
    let ear = Ear::default();
    let take = AssetId::new("take");
    let done = transcribe(&project, &dir, &take, &asked(false), &ear, decode).expect("as set up");
    assert_eq!(done.file.as_str(), "assets/take.words.json");
    assert_eq!(done.heard.as_ref().map(|h| h.language.as_str()), Some("pt"));
    let asset = project.asset(&take).expect("as set up");
    assert_eq!(Words::of(asset, &dir), Some(done.words.clone()));
    assert!(done.said().contains("1 word"), "{}", done.said());

    // Corrected since: a second call reads it back and asks nobody.
    let mut fixed = done.words.clone();
    fixed.words[0].text = "Corta.".into();
    std::fs::write(dir.join("assets/take.words.json"), fixed.to_json()).expect("as set up");
    let kept = transcribe(&project, &dir, &take, &asked(false), &ear, decode).expect("as set up");
    assert_eq!((&kept.heard, &kept.words), (&None, &fixed));
    assert_eq!(ear.asked.get(), 1);
    assert!(
        kept.said().contains("again"),
        "says how to hear it afresh: {}",
        kept.said()
    );

    let again = transcribe(&project, &dir, &take, &asked(true), &ear, decode).expect("as set up");
    assert_eq!(again.words.words[0].text, "Corta!");
    assert_eq!(ear.asked.get(), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn only_an_imported_recording_with_its_file_is_transcribed() {
    let (dir, project) = folder("refusals");
    let ear = Ear::default();
    let refused = |id: &str| {
        transcribe(
            &project,
            &dir,
            &AssetId::new(id),
            &asked(false),
            &ear,
            decode,
        )
        .expect_err("refused")
        .to_string()
    };
    assert_eq!(refused("nope"), "there is no asset `nope`");
    assert!(
        refused("still").contains("`still` is image"),
        "{}",
        refused("still")
    );
    assert!(
        refused("vo").contains("word timings of its own"),
        "{}",
        refused("vo")
    );
    assert!(
        refused("gone").contains("assets/gone.wav"),
        "{}",
        refused("gone")
    );
    assert_eq!(ear.asked.get(), 0);
    std::fs::remove_dir_all(&dir).ok();
}
