//! The panels, rendered offscreen and held to reference images.
//!
//! Six panels were built before this existed and **not one frame of any of
//! them had ever been looked at**, because the window could only be drawn by
//! an event loop and this machine has no display. Every test until now
//! deliberately covered the logic *behind* the drawing, because the drawing
//! was unreachable.
//!
//! `egui_kittest` reaches it: it drives the window's `draw` through wgpu with
//! no window and no display, and hands back an image. So the same argument
//! `docs/golden-renders.md` makes about renders applies here — a GPU
//! rasterising text is no more deterministic than an encoder, so these compare
//! **with tolerance** and never byte-for-byte.
//!
//! And the rule that matters carries over unchanged: **re-blessing a reference
//! to make a test pass is never legitimate.** A snapshot changes when the
//! interface was meant to change, and the new picture is looked at before it is
//! committed.
//!
//! **The line is drawn per platform, and macOS's is wider** (#597). The
//! references are blessed on Linux and CI compares them there, and a Mac draws
//! every panel a little differently. Measured on Apple silicon through Metal,
//! each of the twelve differs from its reference on 2,742 to 30,963 pixels,
//! never by more than two levels in any channel. Under the Linux line that left
//! about a hundred pixels a panel over it, and whether a panel *failed* came
//! down to whether dify's anti-aliasing heuristic happened to excuse every one
//! of them: five were left with 1 to 4 pixels over, seven with none. The seven
//! were exactly as close to failing as the five. [`drawing`] has the numbers.
//!
//! `docs/golden-renders.md` settled the same question for renders by skipping
//! off Linux, and its reason does not carry over. It refused a wider tolerance
//! because that harness has **one** tolerance for every platform: widening it
//! for macOS widened it on Linux too, and a gate that forgives three levels
//! cannot catch a three-level grade bug. `egui_kittest` takes its line per
//! operating system, so macOS's is wider and Linux's is exactly the number that
//! gated before — the Linux gate gives up nothing. What macOS gives up is a
//! shift of two levels or less, which no picture here is about: these assert
//! what is on screen and where, and every such change scores in the thousands.
//! Skipping would have bought the same green by checking nothing, and a Mac
//! that drew a panel wrong would have found out on CI instead.
//!
//! What would reopen it is a Mac drifting past two levels. Measure it as this
//! was measured — every panel at zero tolerance, per-pixel scores — rather than
//! nudging the number; if the drift is no longer a rounding difference, skip
//! off Linux the way `crates/golden` does. Windows keeps the Linux line, since
//! nobody runs these there.
//!
//! Drawing through a GPU also means these can fail by never finishing, which
//! no other test here can do. [`watchdog`] is why they no longer do, and
//! [`drawing`] is the harness they all start from.

mod drawing;
mod fixture;
mod framing;
mod generating;
mod rendering;
mod sequence;
mod watchdog;

use drawing::window;
use framing::Part;

/// The window before anything is open — the first thing anyone sees, and the
/// one state that has to invite rather than look broken. The invitation is the
/// middle of the window, so that is the picture.
#[test]
fn nothing_open() {
    let mut harness = window(None);
    harness.run();
    harness.snapshot(Part::Centre, "nothing_open");
}

/// A whole edit: a title over a colour, music under narration, a duck already
/// written, and two clips nobody has generated.
#[test]
fn a_whole_edit() {
    let project = fixture::project("whole");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    harness.snapshot(Part::Window, "a_whole_edit");
}

/// The same edit in the light theme, with a clip selected.
///
/// Every other picture here is the dark theme, because that is what a window
/// with no choice and no system preference shows — so this is the one that
/// holds the light palette to anything (#643). A clip is selected so the
/// selection ring, the inspector's fields and the outline controls are all in
/// it, which are the parts a palette swap most easily gets wrong.
#[test]
fn a_whole_edit_in_light() {
    let project = fixture::project("light");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().show_in(egui::Theme::Light);
    harness.state_mut().select("c-title");
    harness.run();
    harness.snapshot(Part::Window, "a_whole_edit_in_light");
}

/// A clip selected: the inspector stops saying "select a clip" and starts
/// saying what one is.
#[test]
fn a_clip_selected() {
    let project = fixture::project("selected");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-title");
    harness.run();
    harness.snapshot(Part::Side, "a_clip_selected");
}

/// Several clips selected, on two tracks: every one of them outlined in the
/// timeline, and an inspector that says how many and how far they reach rather
/// than pretending a field could be about all three.
#[test]
fn several_clips_selected() {
    let project = fixture::project("several");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-shot");
    harness.state_mut().also_select("c-title");
    harness.state_mut().also_select("c-vo");
    harness.run();
    // Two pictures, because the claim is about two panels: a crop around both
    // would be the bounding box of an L, which is most of the window.
    harness.snapshot(Part::Timeline, "several_clips_selected_in_the_timeline");
    harness.snapshot(Part::Side, "several_clips_selected_in_the_inspector");
}

/// A scale in flight: the clips drawn where the pointer has put them, and the
/// timeline saying what factor that is and how to keep or cancel it.
///
/// The only state in this window that exists **only while a hand is on it**, so
/// a snapshot is the only way anybody ever looks at it. The pointer is 120
/// pixels left of where the gesture was armed, which is a factor of about 0.71
/// — a cut being tightened, which is the direction people reach for.
#[test]
fn a_scale_in_flight() {
    let project = fixture::project("pacing");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-shot");
    harness.state_mut().also_select("c-title");
    harness.run();
    harness.hover_at(egui::pos2(500.0, 700.0));
    harness.run();
    harness.key_press(egui::Key::S);
    harness.run();
    harness.hover_at(egui::pos2(380.0, 700.0));
    harness.run();
    harness.snapshot(Part::Timeline, "a_scale_in_flight");
}

/// The timeline zoomed out past the end of the film.
///
/// Two things are only visible here. The **end of the edit** — everything past
/// the last frame anything occupies is knocked back and a line is drawn at the
/// edge, so how much of the strip is the actual cut is something you see rather
/// than work out from the ruler. And the **ruler choosing a coarser step**: at
/// this magnification a label a second would be a smear, so it steps up the
/// ladder and keeps its minor ticks between.
///
/// Reached with the zoom key, which is also the only picture in this suite that
/// shows one of the new keys doing anything at all. The view is the timeline's
/// own and nothing outside it can read a magnification off — so a snapshot is
/// the only way anybody looks at this.
#[test]
fn the_timeline_zoomed_out_past_the_end() {
    let project = fixture::project("beyond");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    for _ in 0..8 {
        harness.key_press(egui::Key::Minus);
        harness.run();
    }
    harness.snapshot(Part::Timeline, "the_timeline_zoomed_out_past_the_end");
}

/// A project that does not validate, **open**: the timeline, the pool and the
/// inspector drawn and inert, `read-only` beside the project's name, and every
/// problem at once where the preview would be.
///
/// The picture is the point. This used to be an empty window with a list in it,
/// indistinguishable from picking a folder that was never a project — and
/// seeing which clip is the broken one is the reason somebody opens a broken
/// film in an editor.
#[test]
fn a_project_that_does_not_validate() {
    let project = fixture::broken("broken");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    assert!(
        harness.state().showing().is_some(),
        "it has to really be open, not drawn around"
    );
    harness.snapshot(Part::Centre, "a_project_that_does_not_validate");
    harness.snapshot(Part::Bar, "a_project_that_does_not_validate_in_the_bar");
}

/// A `project.json` that is not JSON. Nothing parsed, so there is no document
/// to show and no clip to point at, and the window says so and stops — which is
/// what makes the state above a different one rather than the same one.
#[test]
fn a_document_that_will_not_parse() {
    let project = fixture::unparseable("unparseable");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    assert!(
        harness.state().showing().is_none(),
        "there is no document to show"
    );
    harness.snapshot(Part::Centre, "a_document_that_will_not_parse");
}

/// A generated shot selected: the brief under the clip's own fields, every
/// optional field marked as one, and — the point of the panel — a length that
/// says why it is not a choice rather than accepting a value and failing later.
#[test]
fn a_generated_shot_selected() {
    let project = fixture::project("brief");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-shot");
    harness.run();
    harness.snapshot(Part::Side, "a_generated_shot_selected");
}

/// The same shot in a window too short for its brief: the inspector scrolls
/// rather than running under the timeline, and the side column ends where the
/// window does (#681).
///
/// At 800 points the brief only just fitted, which is why nobody saw that the
/// inspector had no scroll at all. 500 is short enough that the files list's
/// room would be more than half the column, so this is also the picture of the
/// inspector keeping its half. Scrolled to the bottom, so the picture shows the
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
    harness.hover_at(egui::pos2(1100.0, 120.0));
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -2000.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    // A wheel scroll is animated, so it takes frames rather than one run.
    harness.run_steps(60);
    harness.snapshot(Part::Side, "a_generated_shot_selected_in_a_short_window");
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
    harness.snapshot(Part::Side, "a_narration_line_selected");
}
