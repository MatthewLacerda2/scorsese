//! The generate dialog: the one moment in this window that is not an editing
//! operation, because money leaves.

use super::drawing::window;
use super::fixture;
use super::framing::Part;

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
    harness.snapshot(Part::Dialog("Generate"), "the_generate_dialog");
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
    harness.snapshot(
        Part::Dialog("Generate"),
        "the_generate_dialog_with_narration_priced",
    );
}
