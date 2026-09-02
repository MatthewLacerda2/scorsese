//! The navigation keyboard, driven through real key events.
//!
//! Through the real `draw` rather than by calling the handler, because the half
//! of this that can be wrong is not the arithmetic. Which keys the window
//! listens for, whether a press reaches the playhead the timeline and the
//! preview both read, and whether the ends of the edit hold — none of that is
//! visible from a function that adds one to a frame count.
//!
//! What is deliberately **not** here is `Space`. Starting the transport spawns
//! the mixer and then advances the playhead off a clock, so anything asserted
//! about it afterwards is a race against a thread. The button under the preview
//! and this key call one method between them, and the method is what the rest
//! of the suite covers.

#[path = "../panels/fixture.rs"]
mod fixture;

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use scorsese_app::Scorsese;

/// The last frame of the fixture edit: its longest clip runs 420 frames from
/// zero, and the last frame is the one below that.
const LAST: u64 = 419;
/// A second, on the fixture's own grid.
const SECOND: u64 = 30;

/// A window on `project`, on the machine [`fixture::machine`] states.
fn window(project: &std::path::Path) -> Harness<'static, Scorsese> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, window: &mut Scorsese| window.draw(ui),
            Scorsese::opening_with(Some(project.to_path_buf()), fixture::machine()),
        );
    harness.run();
    harness
}

/// One press, and where the playhead ended up.
fn press(harness: &mut Harness<'static, Scorsese>, key: Key) -> u64 {
    harness.key_press(key);
    harness.run();
    harness.state().playhead().get()
}

/// The same, with shift held.
fn press_shifted(harness: &mut Harness<'static, Scorsese>, key: Key) -> u64 {
    harness.key_down_modifiers(Modifiers::SHIFT, key);
    harness.key_up_modifiers(Modifiers::SHIFT, key);
    harness.run();
    harness.state().playhead().get()
}

/// The commonest thing anybody does in a preview, and the reason the key exists
/// at all: whether a cut lands one frame early is the question a step answers.
#[test]
fn an_arrow_steps_exactly_one_frame_in_each_direction() {
    let project = fixture::project("keys-step");
    let mut harness = window(project.path());
    harness.state_mut().scrub(scorsese_core::Frames(100));

    assert_eq!(press(&mut harness, Key::ArrowRight), 101);
    assert_eq!(press(&mut harness, Key::ArrowRight), 102);
    assert_eq!(press(&mut harness, Key::ArrowLeft), 101);
}

/// Shift means a second, at whatever rate the project is authored at — a jump
/// of a fixed number of frames would be a different distance in every project.
#[test]
fn shift_steps_a_second_of_the_projects_own_grid() {
    let project = fixture::project("keys-second");
    let mut harness = window(project.path());
    harness.state_mut().scrub(scorsese_core::Frames(100));

    assert_eq!(press_shifted(&mut harness, Key::ArrowRight), 100 + SECOND);
    assert_eq!(press_shifted(&mut harness, Key::ArrowLeft), 100);
}

/// Both ends of the film, from a key rather than by dragging a knob to within a
/// pixel of the end of a bar.
#[test]
fn home_and_end_stand_on_the_first_frame_and_the_last() {
    let project = fixture::project("keys-ends");
    let mut harness = window(project.path());

    assert_eq!(press(&mut harness, Key::End), LAST);
    assert_eq!(press(&mut harness, Key::Home), 0);
}

/// There is no picture of an instant the film does not contain, in either
/// direction. Stepping past either end parks rather than running on.
#[test]
fn stepping_past_either_end_parks_there() {
    let project = fixture::project("keys-clamp");
    let mut harness = window(project.path());

    assert_eq!(press(&mut harness, Key::Home), 0);
    assert_eq!(press(&mut harness, Key::ArrowLeft), 0, "before the first");
    assert_eq!(press_shifted(&mut harness, Key::ArrowLeft), 0);

    assert_eq!(press(&mut harness, Key::End), LAST);
    assert_eq!(press(&mut harness, Key::ArrowRight), LAST, "past the last");
    assert_eq!(press_shifted(&mut harness, Key::ArrowRight), LAST);
}

/// The view keys reach the timeline rather than the playhead. There is nothing
/// to read the magnification off from out here — the view is the panel's own —
/// so what this asserts is the half that would otherwise go unnoticed: pressing
/// them moves *nothing else*, which is what a person means by a key that zooms.
#[test]
fn the_view_keys_leave_the_playhead_where_it_is() {
    let project = fixture::project("keys-view");
    let mut harness = window(project.path());
    harness.state_mut().scrub(scorsese_core::Frames(200));

    for key in [Key::Plus, Key::Equals, Key::Minus, Key::F] {
        assert_eq!(press(&mut harness, key), 200, "{key:?} moved the playhead");
    }
}
