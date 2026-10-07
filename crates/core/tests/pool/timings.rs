//! A generated line's word timings leave with it.

use crate::common::new_project;
use scorsese_core::pool::remove_assets;
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

    let report = remove_assets(&mut project, &dir, &[AssetId::new("vo")]).expect("collect");

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

    let report = remove_assets(&mut project, &dir, &[AssetId::new("vo")]).expect("collect");

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

    let report = remove_assets(&mut project, &dir, &[AssetId::new("vo")]).expect("collect");

    assert_eq!(report.files_missing, 1);
    assert!(!Words::beside(&audio).resolve(&dir).exists());
}
