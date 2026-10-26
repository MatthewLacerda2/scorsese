//! What stops a transcription before the server hears it: a recording past
//! the longest, and a stop while it decodes.

use std::path::Path;

use scorsese_core::AssetId;
use scorsese_providers::transcribe::{Hearing, TranscribeError, Undecoded, transcribe};

use super::run::{Ear, asked, folder, never};

/// Past the longest heard, a recording is refused from what its import
/// probed, before anything is decoded or sent.
#[test]
fn a_recording_past_two_hours_is_refused_before_it_is_decoded() {
    let (dir, mut project) = folder("long");
    let take = AssetId::new("take");
    let asset = project
        .assets
        .iter_mut()
        .find(|asset| asset.id == take)
        .expect("as set up");
    asset.media = Some(scorsese_core::MediaMetadata {
        duration_seconds: Some(2.0 * 60.0 * 60.0 + 1.0),
        ..scorsese_core::MediaMetadata::default()
    });
    let ear = Ear::default();
    let refused = transcribe(
        &project,
        &dir,
        &take,
        Hearing {
            asked: &asked(false),
            transcriber: &ear,
            decode: |_: &Path| -> Result<Vec<u8>, Undecoded> { panic!("decoded") },
            stop: &never,
        },
    )
    .expect_err("refused")
    .to_string();
    assert!(
        refused.contains("121 min, longer than the 120 min"),
        "{refused}"
    );
    assert_eq!(ear.asked.get(), 0);
    std::fs::remove_dir_all(&dir).ok();
}

/// A stop while it decodes is a stop, and writes nothing.
#[test]
fn a_decode_stopped_writes_nothing() {
    let (dir, project) = folder("stopped");
    let ear = Ear::default();
    let refused = transcribe(
        &project,
        &dir,
        &AssetId::new("take"),
        Hearing {
            asked: &asked(false),
            transcriber: &ear,
            decode: |_: &Path| -> Result<Vec<u8>, Undecoded> { Err(Undecoded::Stopped) },
            stop: &never,
        },
    )
    .expect_err("stopped");
    assert!(matches!(refused, TranscribeError::Stopped), "{refused:?}");
    assert!(!dir.join("assets/take.mp4.words.json").exists());
    std::fs::remove_dir_all(&dir).ok();
}
