//! A font **file** the project carries that the render would refuse — an
//! italic, a weight it cannot be set at, a file that is not a font, or one
//! that is not there (#1193).
//!
//! A font path is not an asset, so the media pass never sees it: before this,
//! each of these passed `check` clean and failed at `render`.

use crate::common::documents::{clip, project};
use crate::common::run_in;

/// A project whose title is set in `assets/face.ttf`, with `extra` style
/// fields; the file holds `font` when there is one.
fn checked(label: &str, font: Option<&[u8]>, extra: &str) -> crate::common::Run {
    let title = format!(
        r#"{{ "id": "title", "kind": "text", "text": "Hello",
              "style": {{ "font": "assets/face.ttf"{extra} }} }}"#
    );
    let dir = project(label, &[title], &[clip("c1", "title", 0)]);
    if let Some(bytes) = font {
        std::fs::create_dir_all(dir.join("assets")).expect("assets/");
        std::fs::write(dir.join("assets/face.ttf"), bytes).expect("write the font");
    }
    run_in(&dir, &["check"])
}

/// A static face with one weight, which the compositor's tests carry.
fn space_mono() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../compositor/tests/fonts/SpaceMono-Regular.ttf"
    );
    std::fs::read(path).expect("read the test font")
}

#[test]
fn an_italic_of_a_font_file_is_a_problem() {
    let run = checked("file-italic", Some(&space_mono()), r#", "italic": true"#);
    assert!(run.failed, "the render refuses this:\n{}", run.output);
    run.says("`assets/face.ttf` cannot be set as asked");
    run.says("name the italic file itself instead");
}

#[test]
fn a_weight_on_a_static_file_is_a_problem() {
    let run = checked("file-weight", Some(&space_mono()), r#", "weight": 700"#);
    assert!(run.failed, "the render refuses this:\n{}", run.output);
    run.says("cannot be set at 700");
}

#[test]
fn a_file_that_is_not_a_font_is_a_problem() {
    let run = checked("file-junk", Some(b"not a font"), "");
    assert!(run.failed, "the render refuses this:\n{}", run.output);
    run.says("this is not a font scorsese can read");
}

#[test]
fn a_font_file_that_is_not_there_is_one_problem() {
    let run = checked("file-missing", None, "");
    assert!(run.failed, "the render refuses this:\n{}", run.output);
    run.says("there is no font file at that path in the project");
    assert_eq!(
        run.output.matches("problem: ").count(),
        1,
        "one finding:\n{}",
        run.output
    );
}

#[test]
fn a_font_file_set_as_it_can_be_says_nothing() {
    let run = checked("file-fine", Some(&space_mono()), "");
    assert!(!run.failed, "{}", run.output);
    run.silent_about("cannot be set as asked");
}
