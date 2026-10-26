//! Correcting a kept transcript: written back, each word at its times.

use scorsese_core::AssetId;
use scorsese_core::words::{Fix, Words};
use scorsese_providers::transcribe::{correct, transcribe};

use super::run::{Ear, asked, folder, hearing};

#[test]
fn a_correction_is_written_back_and_a_recording_never_heard_is_refused() {
    let (dir, project) = folder("corrects");
    let take = AssetId::new("take");
    let refused = correct(&project, &dir, &take, &[])
        .expect_err("not transcribed yet")
        .to_string();
    assert!(refused.contains("transcribe it first"), "{refused}");
    transcribe(
        &project,
        &dir,
        &take,
        hearing(&asked(false), &Ear::default()),
    )
    .expect("transcribed");
    let fixes = [Fix {
        word: 1,
        text: "Corta.".into(),
    }];
    let fixed = correct(&project, &dir, &take, &fixes).expect("corrected");
    assert_eq!((fixed.fixed, fixed.dropped), (1, 0));
    assert_eq!(fixed.transcript.words.words[0].text, "Corta.");
    let asset = project.asset(&take).expect("as set up");
    assert_eq!(Words::of(asset, &dir), Some(fixed.transcript.words.clone()));
    assert!(
        fixed
            .said()
            .starts_with("`take` corrected in assets/take.mp4.words.json: 1 fix, 0 words dropped"),
        "{}",
        fixed.said()
    );
    let wrong = [Fix {
        word: 2,
        text: "x".into(),
    }];
    let refused = correct(&project, &dir, &take, &wrong)
        .expect_err("one word")
        .to_string();
    assert!(refused.contains("there is no word 2"), "{refused}");
    let refused = correct(&project, &dir, &AssetId::new("still"), &fixes)
        .expect_err("a picture")
        .to_string();
    assert!(refused.contains("`still` is image"), "{refused}");
    std::fs::remove_dir_all(&dir).ok();
}
