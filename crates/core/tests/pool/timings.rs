//! A generated line's word timings leave with it, and so does a transcript.

use crate::common::new_project;
use scorsese_core::pool::{Paid, remove_assets};
use scorsese_core::words::{Word, Words};
use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, ProjectPath};

/// Adds generated line `id` to the project, its audio written under
/// `generated/` and, when `timed`, its timings beside it.
fn generated_line(
    dir: &std::path::Path,
    project: &mut scorsese_core::Project,
    id: &str,
    timed: bool,
) -> ProjectPath {
    let audio = ProjectPath::new(format!("generated/{id}-abc.mp3"));
    std::fs::create_dir_all(dir.join("generated")).expect("make generated/");
    std::fs::write(audio.resolve(dir), b"spoken audio").expect("write the audio");
    if timed {
        let words = Words {
            words: vec![Word {
                text: "Hello.".into(),
                start: 0.0,
                end: 0.5,
            }],
        };
        std::fs::write(Words::beside(&audio).resolve(dir), words.to_json())
            .expect("write the timings");
    }
    let mut line = Asset::sketch(AssetId::new(id), AssetKind::GeneratedAudio, "a line");
    line.state = Some(GenerationState::Generated);
    line.path = Some(audio.clone());
    project.assets.push(line);
    audio
}

#[test]
fn removing_a_timed_line_leaves_neither_file() {
    let (dir, mut project) = new_project("gc-timed");
    let audio = generated_line(&dir, &mut project, "vo", true);
    let timings = Words::beside(&audio).resolve(&dir);
    assert!(timings.is_file());

    let report =
        remove_assets(&mut project, &dir, &[AssetId::new("vo")], Paid::Delete).expect("collect");

    assert!(!audio.resolve(&dir).exists(), "the audio is gone");
    assert!(!timings.exists(), "and its timings with it");
    assert_eq!(report.files_deleted, 1, "the timings go with the audio");
    let timed_bytes = b"spoken audio".len() as u64;
    assert!(
        report.bytes_freed > timed_bytes,
        "the timings' bytes count too"
    );
}

#[test]
fn a_line_without_timings_still_collects() {
    // Generated before timings were kept, or its timings deleted by hand.
    let (dir, mut project) = new_project("gc-untimed");
    let audio = generated_line(&dir, &mut project, "vo", false);

    let report =
        remove_assets(&mut project, &dir, &[AssetId::new("vo")], Paid::Delete).expect("collect");

    assert!(!audio.resolve(&dir).exists());
    assert_eq!(report.files_deleted, 1);
    assert_eq!(report.bytes_freed, b"spoken audio".len() as u64);
}

#[test]
fn timings_go_even_if_the_audio_was_already_gone() {
    // The audio already gone by hand still takes its orphaned timings along.
    let (dir, mut project) = new_project("gc-timings-only");
    let audio = generated_line(&dir, &mut project, "vo", true);
    std::fs::remove_file(audio.resolve(&dir)).unwrap();

    let report =
        remove_assets(&mut project, &dir, &[AssetId::new("vo")], Paid::Delete).expect("collect");

    assert_eq!(report.files_missing, 1);
    assert!(!Words::beside(&audio).resolve(&dir).exists());
}

#[test]
fn an_imported_recordings_transcript_leaves_with_it() {
    let (dir, mut project) = new_project("gc-transcript");
    let take = ProjectPath::new("assets/take.mp4");
    std::fs::create_dir_all(dir.join("assets")).expect("make assets/");
    std::fs::write(take.resolve(&dir), b"footage").expect("write the footage");
    let transcript = Words::transcript(&take).resolve(&dir);
    std::fs::write(&transcript, Words::default().to_json()).expect("write the transcript");
    project.assets.push(Asset::imported(
        AssetId::new("take"),
        AssetKind::Video,
        take.clone(),
    ));
    // The recorder's sound of the same take, under the same name.
    let sound = ProjectPath::new("assets/take.wav");
    std::fs::write(sound.resolve(&dir), b"sound").expect("write the sound");
    let kept = Words::transcript(&sound).resolve(&dir);
    std::fs::write(&kept, Words::default().to_json()).expect("write its transcript");
    project.assets.push(Asset::imported(
        AssetId::new("sound"),
        AssetKind::Audio,
        sound,
    ));

    remove_assets(&mut project, &dir, &[AssetId::new("take")], Paid::Keep).expect("collect");

    assert!(!take.resolve(&dir).exists());
    assert!(!transcript.exists(), "a transcript of nothing is not kept");
    assert!(kept.is_file(), "the sound's own transcript stays");
}
