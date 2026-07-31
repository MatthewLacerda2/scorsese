//! Variable fonts, and the weight a document has to name to use one.
//!
//! The file under test is Manrope, whose `wght` axis runs 200–800 and whose own
//! default instance is **200**. That default is the reason all of this exists:
//! a build that reads a variable file and draws at the default location sets a
//! title card in something near hairline, and every other test in this
//! repository would still pass.

use scorsese_compositor::text::{self, Font, FontError};
use scorsese_core::Rgba;

use crate::ink::{self, canvas, style};

/// A real variable font, unmodified. Where it came from and why it is this one
/// rather than any other is in `crates/compositor/tests/fonts/README.md`.
const VARIABLE: &[u8] = include_bytes!("../fonts/Manrope[wght].ttf");

/// A static file, to prove every rule below is about the *file* and not about
/// the weight being present or absent on its own.
const STATIC: &[u8] = include_bytes!("../../fonts/LiberationSans-Regular.ttf");

fn set_at(weight: u16) -> scorsese_compositor::Frame {
    let font = Font::from_bytes(VARIABLE, Some(weight)).expect("Manrope reaches this weight");
    let mut frame = canvas();
    text::draw(&mut frame, "AB", &font, &style(64.0, Rgba::WHITE));
    frame
}

/// The trap itself. Reading this file with nothing said about weight used to
/// produce ExtraLight and no message; now it produces a message and no render.
#[test]
fn a_variable_font_with_no_weight_named_is_refused() {
    let error = Font::from_bytes(VARIABLE, None).expect_err("must refuse");

    assert!(
        matches!(error, FontError::VariableWithoutWeight { default, .. } if default == 200.0),
        "the refusal should carry the file's own default, since that is the weight \
         that would silently have been used; got `{error}`"
    );
    let message = error.to_string();
    assert!(
        message.contains("variable") && message.contains("weight"),
        "the message has to say what is wrong and what to write; got `{message}`"
    );
}

/// One file, two faces — the other half of the issue, and the reason refusing
/// was affordable. Without this a project wanting bold titles and regular
/// captions carries two near-identical copies of one family.
#[test]
fn one_file_sets_two_different_weights() {
    let (light, heavy) = (set_at(200), set_at(800));

    assert!(ink::count(&light) > 0 && ink::count(&heavy) > 0);
    assert!(
        ink::count(&heavy) > ink::count(&light),
        "800 has to put down more ink than 200; {} against {}",
        ink::count(&heavy),
        ink::count(&light)
    );
}

/// A weight between two the designer drew is a position on the axis, not the
/// nearer of the two — which is what "variable" means and what instancing a
/// file with `fonttools` outside the project used to be needed for.
#[test]
fn a_weight_the_designer_never_drew_still_sets() {
    let (light, middle, heavy) = (set_at(200), set_at(500), set_at(800));
    let ink = |frame| ink::count(frame);

    assert!(
        ink(&light) < ink(&middle) && ink(&middle) < ink(&heavy),
        "500 should sit between 200 and 800; {}, {}, {}",
        ink(&light),
        ink(&middle),
        ink(&heavy)
    );
}

/// Refused rather than clamped to 800. Clamping would be the same silent
/// substitution as the default instance, one step further along.
#[test]
fn a_weight_the_axis_does_not_reach_is_refused_with_the_range() {
    let error = Font::from_bytes(VARIABLE, Some(900)).expect_err("Manrope stops at 800");

    assert!(
        matches!(error, FontError::WeightOffAxis { min, max, .. } if min == 200.0 && max == 800.0),
        "the refusal should name the range the file does reach; got `{error}`"
    );
}

/// A static face has one weight. Ignoring the field would look exactly like
/// honouring it, which is how someone comes to insist their bold is broken.
#[test]
fn a_weight_named_for_a_static_font_is_refused() {
    let error = Font::from_bytes(STATIC, Some(700)).expect_err("Liberation Sans is static");

    assert!(
        matches!(error, FontError::StaticWithWeight { weight: 700 }),
        "got `{error}`"
    );
    assert!(
        error.to_string().contains("static"),
        "the message has to say the file is the reason; got `{error}`"
    );
}
