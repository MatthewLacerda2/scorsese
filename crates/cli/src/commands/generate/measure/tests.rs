//! A line that comes back shorter than the clip laid over its sketch (#1114).

use std::path::PathBuf;

use scorsese_core::testing::StubProbe;
use scorsese_core::{AssetId, MediaMetadata, PROJECT_FILE_NAME, Project, SCHEMA_VERSION};

use super::fitted;

/// Reads every file as one second of sound, which is what the vendor sent.
fn one_second() -> StubProbe {
    StubProbe::reporting(MediaMetadata {
        duration_seconds: Some(1.0),
        ..MediaMetadata::default()
    })
}

/// A project whose narration has just landed: three seconds laid out for it,
/// its file on disk, and nothing measured yet — the state a run leaves it in.
fn just_spoken(label: &str) -> PathBuf {
    let dir = crate::scratch::fresh(&format!("scorsese-{label}"), "");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("generated")).expect("create generated/");
    std::fs::write(dir.join("generated/vo.mp3"), b"a reading").expect("write the line");
    let document = format!(
        r#"{{ "schema_version": {SCHEMA_VERSION}, "name": "spoken",
              "timeline_fps": {{ "num": 30, "den": 1 }},
              "assets": [ {{ "id": "vo", "kind": "generated_audio",
                             "prompt": "Every city has a night editor.",
                             "state": "generated", "path": "generated/vo.mp3" }} ],
              "tracks": [ {{ "id": "a1", "kind": "audio", "clips": [
                  {{ "id": "c-narr", "asset": "vo", "start": 30, "duration": 90 }} ] }} ] }}"#
    );
    std::fs::write(dir.join(PROJECT_FILE_NAME), document).expect("write project.json");
    dir
}

#[test]
fn a_line_shorter_than_its_clip_shortens_the_clip_and_the_project_still_opens() {
    let dir = just_spoken("generate-fit");
    let mut project = Project::load(&dir).expect("open before the probe");

    let shortened =
        fitted(&mut project, &dir, &one_second(), &[AssetId::new("vo")]).expect("measure and save");

    assert_eq!(shortened.len(), 1, "{shortened:?}");
    assert!(
        shortened[0].says().starts_with("c-narr: 90f → 30f"),
        "{}",
        shortened[0].says()
    );
    let reopened = Project::load(&dir).expect("the project still opens");
    let clip = &reopened.tracks[0].clips[0];
    assert_eq!((clip.start.get(), clip.duration.get()), (30, 30));
}

/// Only what this run brought back is fitted: a clip some other edit left
/// overlong is reported by validation, never quietly repaired here.
#[test]
fn an_asset_this_run_did_not_bring_back_is_left_alone() {
    let dir = just_spoken("generate-fit-other");
    let mut project = Project::load(&dir).expect("open before the probe");

    let shortened = fitted(&mut project, &dir, &one_second(), &[]).expect("measure and save");

    assert!(shortened.is_empty(), "{shortened:?}");
}
