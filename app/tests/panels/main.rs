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
//! Drawing through a GPU also means these can fail by never finishing, which
//! no other test here can do. [`watchdog`] is why they no longer do, and
//! [`drawing`] is the harness they all start from.

mod drawing;
mod fixture;
mod watchdog;

use drawing::window;

/// The window before anything is open — the first thing anyone sees, and the
/// one state that has to invite rather than look broken.
#[test]
fn nothing_open() {
    let mut harness = window(None);
    harness.run();
    harness.snapshot("nothing_open");
}

/// A whole edit: a title over a colour, music under narration, a duck already
/// written, and two clips nobody has generated.
#[test]
fn a_whole_edit() {
    let project = fixture::project("whole");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    harness.snapshot("a_whole_edit");
}

/// A clip selected: the inspector stops saying "select a clip" and starts
/// saying what one is.
#[test]
fn a_clip_selected() {
    let project = fixture::project("selected");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-title");
    harness.run();
    harness.snapshot("a_clip_selected");
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
    harness.snapshot("several_clips_selected");
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
    harness.snapshot("a_scale_in_flight");
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
    harness.snapshot("a_project_that_does_not_validate");
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
    harness.snapshot("a_document_that_will_not_parse");
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
    harness.snapshot("a_generated_shot_selected");
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
    harness.snapshot("a_narration_line_selected");
}

/// The generate dialog: what each unmade shot and each unspoken line would
/// cost, the total, and the two sentences that keep the number honest — that it
/// is our calculation, and what the ceiling is.
///
/// Shots and narration are counted and subtotalled **separately**, because
/// their rates are two orders of magnitude apart: a 96¢ shot and a 2¢ line
/// summed into one figure is a number nobody can act on.
///
/// **The one dialog in this window**, because it is the one moment that is not
/// an editing operation: money leaves. A number on screen before a
/// confirmation is the difference between a decision and a surprise.
#[test]
fn the_generate_dialog() {
    let project = fixture::project("generate");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    harness.state_mut().start_generating();
    harness.run();
    harness.snapshot("the_generate_dialog");
}

/// The same dialog with the narration actually priced.
///
/// Every other reference here shows narration at $0.00, because a line is
/// priced only once it has a voice and no committed fixture may carry a voice
/// id — [`fixture::voiced`] is how this one gets one without committing it. So
/// until this image existed, the arithmetic that crosses two vendors a
/// hundredfold apart had never been looked at: ninety-six cents of picture and
/// one cent of speech, subtotalled apart and then added up.
///
/// `narration_is_not_quoted_as_a_video_shot` proves that cent is the right
/// number. This is the only thing that says it is legible.
#[test]
fn the_generate_dialog_with_narration_priced() {
    let project = fixture::voiced("priced");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    harness.state_mut().start_generating();
    harness.run();
    harness.snapshot("the_generate_dialog_with_narration_priced");
}
