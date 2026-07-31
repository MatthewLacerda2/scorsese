//! `scorsese describe` costs nothing, and that is the claim being tested.
//!
//! It runs with ffmpeg and ffprobe pointed at a binary that does not exist. A
//! command that reached for either would say so — `Tools::discover` complains
//! about ffmpeg by name — so a clean run here is what proves the cheapest
//! review really is free, before a frame has been encoded and before the media
//! even has to be on the machine.

use std::path::PathBuf;
use std::process::Command;

/// Nothing runnable, on any platform this builds for.
const NO_FFMPEG: &str = "scorsese-tests-no-such-ffmpeg";

/// A project directory holding one video clip and one narration sketch under
/// it, written straight to disk — the point is the description, not the media.
fn project(label: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("scorsese-describe-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("assets")).expect("create the project directory");
    std::fs::write(dir.join("project.json"), DOCUMENT).expect("write project.json");
    dir
}

fn describe(dir: &PathBuf, arguments: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_scorsese"))
        .arg("describe")
        .arg("--project")
        .arg(dir)
        .args(arguments)
        .env("SCORSESE_FFMPEG", NO_FFMPEG)
        .env("SCORSESE_FFPROBE", NO_FFMPEG)
        .output()
        .expect("run scorsese describe");
    (
        output.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

#[test]
fn describing_a_cut_needs_no_ffmpeg_and_no_media() {
    let dir = project("free");
    let (succeeded, output) = describe(&dir, &[]);

    assert!(succeeded, "describing should not have failed:\n{output}");
    assert!(
        !output.contains("install ffmpeg"),
        "nothing here should have gone looking for it:\n{output}"
    );
    for expected in [
        "Teaser — 4.00s, 120 frames at 30 fps",
        "0.00–2.00s",
        "shot (video, fit)",
        "vo (card, sketch) \"a voice, later\"",
        "2.00–3.00s",
        "3.00–4.00s",
        "gap — black",
    ] {
        assert!(
            output.contains(expected),
            "expected `{expected}` in:\n{output}"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// A slice of the timeline, described on its own — the same flag `render`
/// takes, so what is reviewed and what is encoded can be the same stretch.
#[test]
fn a_range_describes_only_that_slice() {
    let dir = project("range");
    let (succeeded, output) = describe(&dir, &["--range", "60:"]);

    assert!(succeeded, "{output}");
    assert!(output.contains("timeline frames 60–120"), "{output}");
    assert!(!output.contains("0.00–2.00s"), "{output}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A shot, a narration prompt over its first half, then two seconds of nothing
/// — one of each thing a description has to name.
const DOCUMENT: &str = r#"{
  "schema_version": 9,
  "name": "Teaser",
  "timeline_fps": { "num": 30, "den": 1 },
  "assets": [
    { "id": "shot", "kind": "video", "path": "assets/shot.mp4" },
    { "id": "vo", "kind": "generated_audio", "state": "sketch", "prompt": "a voice, later" }
  ],
  "tracks": [
    {
      "id": "v1",
      "kind": "video",
      "clips": [
        { "id": "c1", "asset": "shot", "start": 0, "duration": 60 },
        { "id": "c2", "asset": "shot", "start": 90, "duration": 30 }
      ]
    },
    {
      "id": "a1",
      "kind": "audio",
      "clips": [{ "id": "vo1", "asset": "vo", "start": 0, "duration": 60 }]
    }
  ]
}
"#;
