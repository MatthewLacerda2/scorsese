//! A character **no face scorsese has** can draw.
//!
//! A warning, never a problem: the render succeeds and every other frame is
//! right. What earns it a line at all is that the failure is otherwise
//! invisible — an unmapped character shapes to `.notdef`, which is dropped
//! **with its advance**, so the line closes up and reads as text nobody wrote
//! rather than as text that failed.
//!
//! The named face is not the question on its own. A character it lacks and the
//! fallback face draws is on the frame, so the finding is about what the whole
//! chain cannot say.

use crate::common::documents::{clip, project};
use crate::common::run_in;

/// A text asset in a named face. Written here rather than through
/// `assets::text` because the face is the whole subject.
fn set_in(id: &str, face: &str, content: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "text", "text": "{content}",
              "style": {{ "font": "{face}" }} }}"#
    )
}

fn checked(label: &str, asset: String) -> crate::common::Run {
    let dir = project(label, &[asset], &[clip("c1", "legend", 0)]);
    run_in(&dir, &["check"])
}

#[test]
fn a_character_the_face_cannot_draw_warns_and_still_passes() {
    // `jetbrains-mono` has no U+2713. The marks in a legend are exactly where
    // this bites: they carry the meaning and they vanish.
    let run = checked(
        "glyph-missing",
        set_in("legend", "jetbrains-mono", "resolvido ✓"),
    );
    assert!(
        !run.failed,
        "a face missing a glyph still renders:\n{}",
        run.output
    );
    run.says("warning: asset `legend`");
    run.says("has no glyph for");
    // The finding is about the whole chain and says so: a fallback face having
    // drawn it would have meant no finding at all.
    run.says("nor does any face scorsese falls back to");
    // The code point beside the character, because half of these are invisible
    // in a terminal and the escape is the searchable half.
    run.says("(U+2713)");
    run.says("dropped, not drawn");
}

#[test]
fn a_face_that_covers_what_it_is_asked_says_nothing() {
    // The other half, and the one that decides whether the warning is worth
    // having: Inter does have the tick, so this must be silent about it.
    let run = checked("glyph-fine", set_in("legend", "inter", "resolvido ✓"));
    assert!(!run.failed, "{}", run.output);
    run.silent_about("has no glyph");
    run.says("no problems, nothing to warn about");
}

#[test]
fn each_missing_character_is_named_once() {
    let run = checked(
        "glyph-repeat",
        set_in("legend", "jetbrains-mono", "✓ um ✓ dois ✓ tres ✗"),
    );
    let line = run
        .output
        .lines()
        .find(|line| line.contains("has no glyph"))
        .expect("the warning is there");
    assert_eq!(
        line.matches("U+2713").count(),
        1,
        "a character repeated in the content is still one finding: {line}"
    );
    assert!(
        line.contains("U+2717"),
        "and the other one is there: {line}"
    );
}

#[test]
fn an_emoji_is_drawn_by_the_fallback_and_so_is_not_a_finding() {
    // Inter has no fire, and it does not need one: the chain has a face that
    // does, so the character reaches the frame. Warning about it would be an
    // objection to something that renders correctly.
    let run = checked("glyph-emoji", set_in("legend", "inter", "Ship it 🔥"));
    assert!(!run.failed, "{}", run.output);
    run.silent_about("has no glyph");
    run.says("no problems, nothing to warn about");
}

#[test]
fn a_newline_is_not_a_glyph_anybody_asked_for() {
    // It is an instruction to the line breaker, never handed to a face, so a
    // face not mapping one says nothing about the face.
    let run = checked("glyph-newline", set_in("legend", "inter", "uma\\nlinha"));
    assert!(!run.failed, "{}", run.output);
    run.silent_about("has no glyph");
}
