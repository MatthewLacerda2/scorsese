//! What a face can and cannot draw.
//!
//! Asserted here, beside the code, rather than only through `scorsese check`
//! that consumes it: a face's coverage is the compositor's own knowledge, and
//! a test for it living one crate away means nothing in this crate pins it
//! down at all.

use scorsese_compositor::text::{Font, Slant};

fn shipped(name: &str) -> Font {
    Font::shipped(name, None, Slant::Upright).expect("a published name resolves")
}

#[test]
fn a_character_the_face_lacks_is_reported() {
    // JetBrains Mono has no U+2713. This is the case the whole check exists
    // for: the tick is silently dropped at draw time, so somebody has to say
    // it before the frame does not.
    assert_eq!(
        shipped("jetbrains-mono").uncovered("resolvido ✓"),
        vec!['✓']
    );
}

#[test]
fn a_face_that_covers_everything_it_is_asked_reports_nothing() {
    assert!(shipped("inter").uncovered("resolvido ✓").is_empty());
    assert!(Font::sans().uncovered("The quick brown fox").is_empty());
}

#[test]
fn each_missing_character_is_named_once_in_the_order_it_appears() {
    // Once, because the finding is about the face and the character, not about
    // how many times the content happens to say it. In order, because the
    // first place it appears is where an author would look.
    let font = shipped("jetbrains-mono");
    assert_eq!(font.uncovered("✗ a ✓ b ✗ c ✓"), vec!['✗', '✓']);
}

#[test]
fn a_newline_is_never_reported() {
    // It is an instruction to the line breaker and is never handed to a face,
    // so a face not mapping one says nothing about the face. Without this the
    // check would warn about every multi-line title in the project.
    let font = shipped("jetbrains-mono");
    assert!(font.uncovered("uma\nlinha").is_empty());
    assert_eq!(font.uncovered("uma\nlinha ✓"), vec!['✓']);
}

#[test]
fn a_face_carried_by_the_project_answers_the_same_way() {
    // The path case, not the shipped-name case: coverage is a property of the
    // bytes, and nothing about the two routes may differ.
    let bytes = std::fs::read("tests/fonts/SpaceMono-Regular.ttf").expect("the test font is there");
    let font = Font::from_bytes(&bytes, None).expect("it parses");
    assert!(font.uncovered("abc").is_empty());
    assert_eq!(font.uncovered("abc \u{10FFFF}"), vec!['\u{10FFFF}']);
}
