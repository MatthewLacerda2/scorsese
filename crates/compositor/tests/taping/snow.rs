//! The snow: the same noise field grain draws, and the colour it carries.

use scorsese_core::{Grade, Vhs};

use super::{SIZE, flat, instant, pixel, taped};

const GREY: [u8; 4] = [128, 128, 128, 255];

const SNOW: Vhs = Vhs {
    noise: 0.5,
    ..Vhs::NONE
};

/// Which of the layer's pixels are not grey — the count that separates snow
/// with a chroma path from snow without one.
fn coloured(frame: &scorsese_compositor::Frame) -> usize {
    (0..SIZE)
        .flat_map(|y| (0..SIZE).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let (r, g, b, _) = pixel(frame, x, y);
            r != g || g != b
        })
        .count()
}

/// **In colour, the snow speckles the colour differences too**, which is what
/// makes tape noise coloured where film grain is not: a flat grey plate comes
/// back with colour in it that nobody put there.
#[test]
fn tape_snow_carries_colour() {
    let frame = taped(&flat(GREY), instant(SNOW, "c1", 0));
    assert!(
        coloured(&frame) > 100,
        "a colour tape's snow puts colour on a grey plate"
    );
}

/// **And in mono it cannot**, because there is no chroma path for it to be laid
/// on. That is the whole of the difference between the two modes, and it is why
/// `mono` is a mode rather than a desaturation applied afterwards — a
/// desaturation would take the colour back out of snow that had already been
/// laid, and land on a different picture.
#[test]
fn mono_snow_carries_none() {
    let frame = taped(&flat(GREY), instant(Vhs { mono: true, ..SNOW }, "c1", 0));
    assert_eq!(coloured(&frame), 0);
    // And there is still snow — a mono tape is not a clean picture.
    assert_ne!(frame.bytes(), flat(GREY).bytes());
}

/// Hashed and not generated, exactly as grain is: one instant is one picture,
/// and the next instant is another.
#[test]
fn the_snow_is_a_function_of_the_clip_and_the_frame() {
    assert_eq!(
        taped(&flat(GREY), instant(SNOW, "c1", 3)).bytes(),
        taped(&flat(GREY), instant(SNOW, "c1", 3)).bytes()
    );
    assert_ne!(
        taped(&flat(GREY), instant(SNOW, "c1", 3)).bytes(),
        taped(&flat(GREY), instant(SNOW, "c1", 4)).bytes()
    );
    assert_ne!(
        taped(&flat(GREY), instant(SNOW, "c1", 3)).bytes(),
        taped(&flat(GREY), instant(SNOW, "c2", 3)).bytes()
    );
}

/// A clip may carry both, and they are two textures rather than one at twice
/// the height. Drawn from one field, the tape's snow and the film's grain would
/// agree pixel for pixel and add up to a single speckle — so this asks that the
/// two together are not the same picture as either alone doubled.
#[test]
fn the_tape_s_snow_and_the_film_s_grain_are_different_fields() {
    let mut both = instant(SNOW, "c1", 0);
    both.grade = Grade {
        grain: 0.5,
        ..Grade::NEUTRAL
    };
    let mut twice = instant(
        Vhs {
            noise: 1.0,
            ..Vhs::NONE
        },
        "c1",
        0,
    );
    twice.grade = Grade::NEUTRAL;
    assert_ne!(
        taped(&flat(GREY), both).bytes(),
        taped(&flat(GREY), twice).bytes()
    );
}
