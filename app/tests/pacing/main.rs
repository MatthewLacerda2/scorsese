//! The pacing gesture, driven the way a hand drives it: a key, then pointer
//! travel, then a key to keep it or a key to take it back.
//!
//! Through the real `draw` and real input events rather than by calling the
//! gesture directly, because half of what can be wrong about a **modal**
//! gesture is the modality — arming when nothing is selected, a click landing
//! on a clip instead of ending the scale, an `Esc` that puts back only some of
//! what it moved. None of that is visible from the arithmetic.

#[path = "../panels/fixture.rs"]
mod fixture;

use egui::{Key, Pos2, pos2};
use egui_kittest::Harness;
use scorsese_app::Scorsese;

/// Where the pointer stands when the gesture is armed.
const START: Pos2 = pos2(500.0, 700.0);

/// 240 pixels to the right of it — the travel `timeline::pacing` says doubles
/// the spacing. Written here as a number so that a change to the feel of the
/// gesture shows up as a test to re-read rather than as a silent pass.
const DOUBLED: Pos2 = pos2(740.0, 700.0);

/// A window on `project`, on the machine [`fixture::machine`] states.
///
/// Nothing here draws a ceiling, so stating it changes no assertion today. It
/// is here because *every* window a test opens should be on a machine somebody
/// wrote down: the alternative is finding out which tests inherited the
/// developer's configuration one failure at a time.
fn window(project: &std::path::Path) -> Harness<'static, Scorsese> {
    Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, window: &mut Scorsese| window.draw(ui),
            Scorsese::opening_with(Some(project.to_path_buf()), fixture::machine()),
        )
}

/// Where a clip sits now, as `(start, duration)`.
fn at(harness: &Harness<'static, Scorsese>, id: &str) -> (u64, u64) {
    harness
        .state()
        .showing()
        .expect("a project is open")
        .clips()
        .find(|(_, clip)| clip.id.as_str() == id)
        .map(|(_, clip)| (clip.start.get(), clip.duration.get()))
        .expect("the clip is in this project")
}

/// The two clips of the picture track, selected, with the playhead at the
/// start — so the whole cut scales from its beginning.
fn holding(harness: &mut Harness<'static, Scorsese>) {
    harness.state_mut().select("c-shot");
    harness.state_mut().also_select("c-title");
    harness.run();
    harness.hover_at(START);
    harness.run();
    harness.key_press(Key::S);
    harness.run();
}

/// The gesture end to end. `c-shot` runs 0–300 and `c-title` 300–420, so they
/// touch; doubled, they must still touch, and both are titles and briefs whose
/// length the timeline owns, so both stretch.
#[test]
fn the_clips_follow_the_pointer_and_the_cut_stays_gapless() {
    let project = fixture::project("pace-follow");
    let mut harness = window(project.path());
    holding(&mut harness);

    harness.hover_at(DOUBLED);
    harness.run();
    assert_eq!(at(&harness, "c-shot"), (0, 600));
    assert_eq!(at(&harness, "c-title"), (600, 240), "and still touching");
}

/// Committing is what puts it on disk — and nothing before it does, because a
/// scale on the way to the one you meant is not an edit anybody wanted kept.
#[test]
fn nothing_reaches_the_disk_until_the_gesture_is_kept() {
    let project = fixture::project("pace-keep");
    let file = project.path().join("project.json");
    let before = std::fs::read_to_string(&file).expect("read");
    let mut harness = window(project.path());
    holding(&mut harness);

    harness.hover_at(DOUBLED);
    harness.run();
    assert_eq!(
        std::fs::read_to_string(&file).expect("read"),
        before,
        "a scale in flight must not have been written"
    );

    harness.key_press(Key::Enter);
    harness.run();
    let after = std::fs::read_to_string(&file).expect("read");
    assert!(after.contains(r#""duration": 600"#), "got {after}");
}

/// One key puts back everything the gesture moved. A scale is one action, so
/// taking back part of it would be worse than not having it.
#[test]
fn escape_puts_the_whole_cut_back() {
    let project = fixture::project("pace-cancel");
    let file = project.path().join("project.json");
    let before = std::fs::read_to_string(&file).expect("read");
    let mut harness = window(project.path());
    holding(&mut harness);

    harness.hover_at(DOUBLED);
    harness.run();
    assert_eq!(at(&harness, "c-shot"), (0, 600), "it has to have moved");

    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(at(&harness, "c-shot"), (0, 300));
    assert_eq!(at(&harness, "c-title"), (300, 120));
    assert_eq!(std::fs::read_to_string(&file).expect("read"), before);
}

/// The key means "scale *these*", so with nothing chosen there is nothing for
/// it to mean — and the window says so rather than looking broken.
#[test]
fn the_key_arms_nothing_while_nothing_is_selected() {
    let project = fixture::project("pace-unarmed");
    let mut harness = window(project.path());
    harness.hover_at(START);
    harness.run();
    harness.key_press(Key::S);
    harness.run();

    harness.hover_at(DOUBLED);
    harness.run();
    assert_eq!(at(&harness, "c-shot"), (0, 300), "nothing was scaled");
}
