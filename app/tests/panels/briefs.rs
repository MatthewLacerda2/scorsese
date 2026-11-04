//! The inspector with a generated asset selected: the brief behind a shot, a
//! line of narration, a track of music and a sound effect, each the panel's own.

use crate::drawing::{self, window};
use crate::fixture;
use crate::framing::Part;

/// A generated shot selected: the brief under the clip's own fields, every
/// optional field marked as one, and — the point of the panel — a length that
/// says why it is not a choice rather than accepting a value and failing later.
#[test]
fn a_generated_shot_selected() {
    let project = fixture::project("brief");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-shot");
    harness.run();
    harness.snapshot(Part::Inspector, "a_generated_shot_selected");
}

/// The same shot in a window too short for its brief: the inspector scrolls
/// rather than running under the timeline, and its column ends where the
/// window does (#681).
///
/// At 800 points the brief only just fitted, which is why nobody saw that the
/// inspector had no scroll at all. Scrolled to the bottom, so the picture shows the
/// last field reached rather than the first ones repeated.
#[test]
fn a_generated_shot_selected_in_a_short_window() {
    let project = fixture::project("short");
    let mut harness = drawing::sized(
        Some(project.path().to_path_buf()),
        egui::vec2(1280.0, 500.0),
    );
    harness.state_mut().select("c-shot");
    harness.run();
    harness.hover_at(egui::pos2(150.0, 120.0));
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -2000.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    // A wheel scroll is animated, so it takes frames rather than one run.
    harness.run_steps(60);
    harness.snapshot(
        Part::Inspector,
        "a_generated_shot_selected_in_a_short_window",
    );
}

/// A generated line selected: the words, what they will cost to speak, and the
/// two states this panel exists to make readable.
///
/// The language is **not offered** on the model that silently ignores it — the
/// reason is on screen, naming the model, rather than arriving as a refusal
/// after somebody has typed a code. And with nothing in the project's voice
/// cache the picker says what is missing and which command fills it, which is
/// what a person meets before they have ever run `scorsese voices`.
#[test]
fn a_narration_line_selected() {
    let project = fixture::project("narration");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-vo");
    harness.run();
    harness.snapshot(Part::Inspector, "a_narration_line_selected");
}

/// A generated track of music selected: named as music, not speech, with its
/// description, its length, whether it is sung, and what that length costs.
#[test]
fn a_music_track_selected() {
    // The narration swapped for a track with a length already set.
    let document = fixture::DOCUMENT
        .replace(
            r#""prompt": "In a city that never sleeps, one editor never blinks." }"#,
            r#""prompt": "slow warm piano under a city waking up",
      "music": { "seconds": 30 } }"#,
        )
        .replace(r#"{ "id": "vo","#, r#"{ "id": "score","#)
        .replace(r#""asset": "vo""#, r#""asset": "score""#);
    let project = fixture::write("scored", &document);
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-vo");
    harness.run();
    harness.snapshot(Part::Inspector, "a_music_track_selected");
}

/// A generated sound effect selected: named as a sound, with its description,
/// a length in fractions of a second, whether it loops, and what it costs.
#[test]
fn a_sound_effect_selected() {
    // The narration swapped for a sound with a length already set.
    let document = fixture::DOCUMENT
        .replace(
            r#""prompt": "In a city that never sleeps, one editor never blinks." }"#,
            r#""prompt": "a heavy wooden door creaking open",
      "sound": { "seconds": 2.5 } }"#,
        )
        .replace(r#"{ "id": "vo","#, r#"{ "id": "door","#)
        .replace(r#""asset": "vo""#, r#""asset": "door""#);
    let project = fixture::write("sounded", &document);
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-vo");
    harness.run();
    harness.snapshot(Part::Inspector, "a_sound_effect_selected");
}
