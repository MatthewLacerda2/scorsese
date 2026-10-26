//! What the suppression takes off the subject, and what it leaves alone.

use scorsese_core::{ChromaKey, Rgba};

use super::{SCREEN, SKIN, SPILLED, keyed, near, pixel, plate};

/// A key with the suppression on, at a tolerance that keeps everything these
/// look at.
fn despilling(color: Rgba) -> ChromaKey {
    ChromaKey {
        color,
        tolerance: 0.25,
        softness: 0.1,
        spill: true,
    }
}

/// The whole of what the boolean is for: a subject with the screen's bounce on
/// it comes back without it.
///
/// `(199, 223, 162)` is the subject under a green bounce — green 24 levels
/// above red where the subject itself is 38 below it. Despilled it is
/// `(216, 218, 167)`: green is now within two levels of red, which is the cast
/// gone rather than reduced. The numbers come from the arithmetic `chroma.rs`
/// documents, applied on paper.
#[test]
fn the_screen_s_bounce_comes_off_the_subject() {
    let keyed = keyed(&plate(), despilling(SCREEN));
    near(pixel(&keyed, 5, 0), (216, 218, 167), "a spilled subject");
}

/// And the light does not leave with it, which is the refinement that stops a
/// despilled edge reading as a grey rim where there was a green one.
///
/// The spilled pixel's Rec.709 luma is 213.5; despilled and put back it is
/// 213.9. Clamping the green without restoring the luma would land at 189 —
/// two dozen levels of brightness gone from exactly the edge pixels a key is
/// most visible on.
#[test]
fn the_despilled_subject_keeps_its_brightness() {
    let keyed = keyed(&plate(), despilling(SCREEN));
    let (r, g, b) = pixel(&keyed, 5, 0);
    let luma = |(r, g, b): (f64, f64, f64)| 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let before = luma((
        f64::from(SPILLED.r),
        f64::from(SPILLED.g),
        f64::from(SPILLED.b),
    ));
    let after = luma((f64::from(r), f64::from(g), f64::from(b)));
    assert!((before - after).abs() < 1.5, "{before} became {after}");
}

/// A colour the scene had is a colour the scene keeps. The subject with no
/// bounce on it carries no more green than its red and blue account for, so
/// nothing is taken off it — which is what stops the suppression being a
/// desaturation of the whole layer.
#[test]
fn a_subject_with_no_bounce_on_it_is_left_alone() {
    let keyed = keyed(&plate(), despilling(SCREEN));
    near(pixel(&keyed, 4, 0), (SKIN.r, SKIN.g, SKIN.b), "the subject");
}

/// And it is not about green. A magenta screen spills magenta, and the same
/// boolean pulls red and blue down toward green instead — the statement being
/// about the component along whatever hue was keyed.
///
/// The control is the second half: a *green* key leaves a magenta-spilled pixel
/// exactly alone, so this is the despill following the key colour rather than
/// touching everything it is pointed at.
#[test]
fn the_suppression_follows_whichever_hue_was_keyed() {
    let magenta = Rgba::opaque(199, 0, 177);
    let spilled = Rgba::opaque(255, 161, 202);
    let mut source = plate();
    for pixel in source.bytes_mut().chunks_exact_mut(4) {
        pixel.copy_from_slice(&spilled.channels());
    }
    near(
        pixel(&keyed(&source, despilling(magenta)), 0, 0),
        (211, 178, 160),
        "a magenta-spilled subject under a magenta key",
    );
    near(
        pixel(&keyed(&source, despilling(SCREEN)), 0, 0),
        (spilled.r, spilled.g, spilled.b),
        "and the same pixel under a green one",
    );
}

/// The boolean is a boolean: with it off, the bounce stays.
#[test]
fn the_bounce_stays_when_nobody_asked_for_it() {
    let keyed = keyed(&plate(), super::key(0.25));
    near(
        pixel(&keyed, 5, 0),
        (SPILLED.r, SPILLED.g, SPILLED.b),
        "a spilled subject with the suppression off",
    );
}
