//! `transcribe` and `transcript_fix`: an imported recording's words, kept
//! beside it and corrected in place (#1549). The Whisper server itself is
//! `scorsese-providers`' to test, behind its trait; what is held here is what
//! the tools refuse before reaching it, and what they say.

use scorsese_core::words::{Word, Words};
use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath};
use serde_json::json;

use super::fixture::project;
use crate::{call, said};

/// The fixture with footage `take` imported, transcribed when `heard`.
fn footage(label: &str, heard: bool) -> std::path::PathBuf {
    let dir = project(label);
    let take = ProjectPath::new("assets/take.mp4");
    std::fs::write(take.resolve(&dir), b"footage").expect("write the footage");
    let mut document = Project::load(&dir).expect("the fixture loads");
    document.assets.push(Asset::imported(
        AssetId::new("take"),
        AssetKind::Video,
        take.clone(),
    ));
    document.save(&dir).expect("save");
    if heard {
        let said = |text: &str, start: f64| Word {
            text: text.into(),
            start,
            end: start + 0.3,
        };
        let words = Words {
            words: vec![said("Olha", 0.0), said("casa", 0.3), said("bê", 0.6)],
        };
        std::fs::write(Words::transcript(&take).resolve(&dir), words.to_json())
            .expect("write the transcript");
    }
    dir
}

#[test]
fn a_kept_transcript_is_read_back_numbered_without_asking_whisper() {
    let dir = footage("transcribe-kept", true);
    let (text, failed) = said(&call(
        "transcribe",
        json!({ "project": dir, "asset": "take" }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.starts_with("`take` was already transcribed: 3 words in assets/take.mp4.words.json"),
        "{text}"
    );
    assert!(text.contains("   2  0.30–0.60  casa"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn what_cannot_be_transcribed_is_refused_before_anything_is_heard() {
    let dir = footage("transcribe-refused", false);
    for (arguments, says) in [
        (json!({ "project": dir }), "asset"),
        (
            json!({ "project": dir, "asset": "nope" }),
            "there is no asset `nope`",
        ),
        (
            json!({ "project": dir, "asset": "title" }),
            "`title` is text",
        ),
        (
            json!({ "project": dir, "asset": "vo" }),
            "word timings of its own",
        ),
        (
            json!({ "project": dir, "asset": "take", "language": "Portuguese" }),
            "like `pt` or `en`",
        ),
    ] {
        let (text, failed) = said(&call("transcribe", arguments));
        assert!(failed, "{text}");
        assert!(text.contains(says), "{says} not in: {text}");
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_misheard_word_is_corrected_keeping_its_times() {
    let dir = footage("transcript-fix", true);
    let fixes = json!([{ "word": 2, "text": "Kassab" }, { "word": 3, "text": "" }]);
    let (text, failed) = said(&call(
        "transcript_fix",
        json!({ "project": dir, "asset": "take", "fixes": fixes }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("1 word dropped"), "{text}");
    assert!(text.contains("   2  0.30–0.60  Kassab"), "{text}");
    let take = Project::load(&dir).expect("loads");
    let words = Words::of(take.asset(&AssetId::new("take")).expect("take"), &dir);
    assert_eq!(words.map(|w| w.words.len()), Some(2));

    let (text, failed) = said(&call(
        "transcript_fix",
        json!({ "project": dir, "asset": "take", "fixes": [{ "word": 9, "text": "x" }] }),
    ));
    assert!(failed);
    assert!(text.contains("there is no word 9"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}
