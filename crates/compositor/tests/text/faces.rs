//! The two shipped faces, and a project's own.

use scorsese_compositor::text::{self, Font};
use scorsese_core::Rgba;

use crate::ink::{self, canvas, style};

/// The same bytes the crate compiles in, read again through the public door —
/// so `from_bytes` is proven to be a real path rather than one only the two
/// statics use.
const SERIF_FILE: &[u8] = include_bytes!("../../fonts/LiberationSerif-Regular.ttf");

fn set_in(font: &Font) -> scorsese_compositor::Frame {
    let mut frame = canvas();
    text::draw(&mut frame, "Chapter One", font, &style(32.0, Rgba::WHITE));
    frame
}

/// A default nobody can select is not a choice: the serif face has to be
/// reachable *and* has to come out different from the sans one.
#[test]
fn the_two_shipped_faces_set_the_same_words_differently() {
    let sans = set_in(Font::sans());
    let serif = set_in(Font::serif());

    assert!(ink::count(&sans) > 0 && ink::count(&serif) > 0);
    assert_ne!(
        sans.bytes(),
        serif.bytes(),
        "asking for the serif face has to change the picture"
    );
}

/// Both faces are metric-compatible with the proprietary ones they stand in
/// for, and with each other in the widths that matter — a title set in one is
/// about as wide as the same title in the other, which is why swapping the
/// face does not re-wrap the cut.
#[test]
fn the_two_faces_set_a_title_to_about_the_same_width() {
    let width = |frame| {
        let (left, _, right, _) = ink::bounds(frame).expect("something was drawn");
        f32::from(u16::try_from(right - left).expect("a raster this size"))
    };
    let (sans, serif) = (width(&set_in(Font::sans())), width(&set_in(Font::serif())));
    let ratio = sans / serif;
    assert!(
        (0.85..=1.15).contains(&ratio),
        "the two faces should set a title to a similar width; {sans} against {serif}"
    );
}

#[test]
fn a_project_can_bring_a_font_of_its_own() {
    let brought = Font::from_bytes(SERIF_FILE, None).expect("a real font file parses");
    assert_eq!(
        set_in(&brought).bytes(),
        set_in(Font::serif()).bytes(),
        "the same face, whether it arrived as bytes or as a shipped default"
    );
}

#[test]
fn something_that_is_not_a_font_is_refused() {
    let error = Font::from_bytes(b"this is not a font", None).expect_err("must refuse");
    assert!(
        error.to_string().contains("not a font"),
        "the message should say what is wrong; got `{error}`"
    );
}
