//! What survives the key, and what does not.

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer};
use scorsese_core::{ChromaKey, Rgba};

use super::{DIM, DIMMER, HEIGHT, SCREEN, SKIN, STRAND, key, keyed, near, pixel, plate, raster};

/// The requirement the chromaticity plane exists for: one key, one screen, and
/// three very different brightnesses of it all gone together.
///
/// A keyer measuring RGB distance passes the first column of this and fails the
/// other two — which is precisely the shape of the bug it would ship, since a
/// screen is always brightest where the lights are pointed.
#[test]
fn an_unevenly_lit_screen_keys_as_one_screen() {
    let keyed = keyed(&plate(), key(0.25));
    for row in 0..HEIGHT {
        for (column, band) in [SCREEN, DIM, DIMMER].into_iter().enumerate() {
            let found = pixel(&keyed, column as u32, row);
            assert_eq!(found, (0, 0, 0), "{band} at row {row} is still there");
        }
    }
}

/// And the other half, which a key that simply erased everything would also
/// satisfy: the subject comes back exactly itself.
#[test]
fn the_subject_comes_back_untouched() {
    let keyed = keyed(&plate(), key(0.25));
    near(pixel(&keyed, 4, 0), (SKIN.r, SKIN.g, SKIN.b), "the subject");
}

/// The ramp, at three tolerances that put one colour on either side of it and
/// in the middle of it.
///
/// The strand sits `0.288` from the screen. With a softness of `0.1`: a
/// tolerance of `0.30` swallows it whole, `0.25` leaves `(0.288 − 0.25) / 0.1`
/// of it — alpha 97 — and `0.20` leaves `0.881` of it, alpha 225. Over black
/// those are the strand's own colour scaled by each, and the three are far
/// enough apart that a ramp running the other way, or one end mistaken for the
/// other, cannot land on any of them.
#[test]
fn an_edge_ramps_between_the_two_thresholds() {
    let scaled = |alpha: u32| {
        let at = |channel: u8| ((u32::from(channel) * alpha + 127) / 255) as u8;
        (at(STRAND.r), at(STRAND.g), at(STRAND.b))
    };
    for (tolerance, expected) in [
        (0.30, (0, 0, 0)),
        (0.25, scaled(97)),
        (0.20, scaled(225)),
        (0.15, (STRAND.r, STRAND.g, STRAND.b)),
    ] {
        near(
            pixel(&keyed(&plate(), key(tolerance)), 3, 0),
            expected,
            &format!("the strand at a tolerance of {tolerance}"),
        );
    }
}

/// A key can only ever take opacity away: a source that arrived half
/// transparent stays at most half transparent where the key kept it.
///
/// Half of the subject's colour over black, not the whole of it — which is what
/// an alpha that *replaced* the source's rather than scaling it would produce.
#[test]
fn a_source_with_alpha_of_its_own_keeps_it() {
    let mut source = plate();
    for pixel in source.bytes_mut().chunks_exact_mut(4) {
        pixel[3] = 128;
    }
    let keyed = keyed(&source, key(0.25));
    let half = |channel: u8| ((u32::from(channel) * 128 + 127) / 255) as u8;
    near(
        pixel(&keyed, 4, 0),
        (half(SKIN.r), half(SKIN.g), half(SKIN.b)),
        "a half-transparent subject",
    );
    assert_eq!(pixel(&keyed, 0, 0), (0, 0, 0), "and the screen still goes");
}

/// A layer with no key is the layer it always was — including down the copy
/// path, which is the one a plate with nothing else on it takes.
#[test]
fn a_layer_with_no_key_is_the_layer_it_always_was() {
    let plate = plate();
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .composite(&mut canvas, &[Layer::plain(&plate)])
        .expect("compositing succeeds");
    assert_eq!(canvas.bytes(), plate.bytes());
}

/// And a keyed one is never taken down that path, which is the failure this
/// costs least to make: a full-frame plate with a key and nothing else
/// satisfies every other condition an identity layer does, so a copy would hand
/// the screen through fully opaque with the key doing nothing at all.
#[test]
fn a_keyed_layer_is_never_copied_through() {
    let keyed = keyed(&plate(), key(0.25));
    assert_ne!(keyed.bytes(), plate().bytes());
}

/// A screen colour with no light in it has no chromaticity to measure from, so
/// there is nothing to key and the layer is left alone rather than keyed
/// against noise. Validation refuses it a step earlier; this is the arithmetic
/// refusing it too.
#[test]
fn a_screen_with_no_colour_keys_nothing() {
    let keyed = keyed(&plate(), ChromaKey::new(Rgba::opaque(0, 0, 0)));
    assert_eq!(keyed.bytes(), plate().bytes());
}
