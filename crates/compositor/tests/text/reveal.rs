//! A block arriving a piece at a time: where the pieces are, and that a
//! finished reveal is the block drawn whole.

use scorsese_compositor::Frame;
use scorsese_compositor::text::{self, Edge, Font, Reveal, Style, Sweep};
use scorsese_core::{Easing, RevealUnit, Rgba};

use crate::ink::{bounds, canvas, count, style};

fn revealed(content: &str, unit: RevealUnit, at: f64, stagger: f64) -> Frame {
    revealed_in(&style(28.0, Rgba::WHITE), content, unit, at, stagger)
}

fn revealed_in(style: &Style, content: &str, unit: RevealUnit, at: f64, stagger: f64) -> Frame {
    let mut frame = canvas();
    let reveal = Reveal {
        unit,
        rise: 0.0,
        stagger,
        sweep: Sweep {
            at,
            easing: Easing::Linear,
            backwards: false,
        },
    };
    text::draw_revealing(&mut frame, content, Font::sans(), style, &reveal);
    frame
}

/// The reveal is laid out as the block is and only then cut up, so at its end
/// it is not a lookalike of the plain drawing — it is the same raster.
#[test]
fn a_finished_reveal_is_the_block_drawn_whole() {
    let content = "Ship it 🔥 now";
    let mut whole = canvas();
    text::draw(&mut whole, content, Font::sans(), &style(28.0, Rgba::WHITE));
    for unit in [RevealUnit::Char, RevealUnit::Word, RevealUnit::Line] {
        assert!(
            revealed(content, unit, 1.0, 0.5).bytes() == whole.bytes(),
            "{unit:?} at 1 is the block itself"
        );
    }
}

#[test]
fn nothing_has_arrived_at_the_start() {
    assert_eq!(
        count(&revealed("Ship it now", RevealUnit::Word, 0.0, 0.5)),
        0
    );
}

/// Halfway through three words one after another, the first word is in and
/// the last has not begun — so the ink stops short of where the line ends,
/// and starts exactly where it starts.
#[test]
fn words_arrive_in_reading_order_without_moving() {
    let whole = bounds(&revealed("Ship it now", RevealUnit::Word, 1.0, 1.0)).expect("ink");
    let half = bounds(&revealed("Ship it now", RevealUnit::Word, 0.5, 1.0)).expect("ink");
    assert_eq!(half.0, whole.0, "the first word is where it will stay");
    assert!(
        half.2 < whole.2,
        "the last word has not arrived: {half:?} {whole:?}"
    );
}

/// A cluster from the fallback face is one piece: halfway through `a🔥b`
/// by character, the fire is partway in as a whole drawing — some of it
/// showing, none of it solid — rather than missing or half-drawn.
#[test]
fn an_emoji_arrives_as_one_piece() {
    let frame = revealed("a🔥b", RevealUnit::Char, 0.5, 1.0);
    let colours = frame
        .bytes()
        .chunks_exact(4)
        .filter(|pixel| pixel[3] > 0 && !(pixel[0] == pixel[1] && pixel[1] == pixel[2]))
        .map(|pixel| pixel[3])
        .collect::<Vec<_>>();
    assert!(!colours.is_empty(), "the fire is on its way in");
    assert!(
        colours.iter().all(|alpha| *alpha < 200),
        "and none of it is solid yet"
    );
}

/// A piece halfway in is the piece drawn whole at half its alpha, pixel for
/// pixel — with a rim too, which is the case that has to be faded as one
/// picture: a rim and a fill faded apart would show the rim through the half
/// of it the fill covers, a dark ring inside every letter.
#[test]
fn a_piece_halfway_in_is_the_piece_whole_at_half_alpha() {
    let plain = style(28.0, Rgba::WHITE);
    let rimmed = Style {
        edge: Some(Edge {
            color: Rgba::BLACK,
            width: 3.0,
        }),
        ..plain
    };
    for style in [plain, rimmed] {
        let whole = revealed_in(&style, "Ship it", RevealUnit::Line, 1.0, 0.0);
        let half = revealed_in(&style, "Ship it", RevealUnit::Line, 0.5, 0.0);
        for (whole, half) in whole
            .bytes()
            .chunks_exact(4)
            .zip(half.bytes().chunks_exact(4))
        {
            let alpha = (f32::from(whole[3]) * 0.5).round() as u8;
            let expected = if alpha == 0 {
                [0; 4]
            } else {
                [whole[0], whole[1], whole[2], alpha]
            };
            let close = half.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1);
            assert!(close, "{:?}: {half:?} for {whole:?}", style.edge);
        }
    }
}

/// Two words partway in at once, each at its own opacity: three quarters and
/// one quarter, halfway through a half-staggered pair. Each word's most solid
/// pixel says which opacity it was drawn at, so neither is drawn at the
/// other's.
#[test]
fn two_pieces_partway_in_each_keep_their_own_opacity() {
    let frame = revealed("Ship it", RevealUnit::Word, 0.5, 0.5);
    let (left, _, right, _) = bounds(&frame).expect("ink");
    // Where the first word ends: a full stagger at halfway has it in alone.
    let split = bounds(&revealed("Ship it", RevealUnit::Word, 0.5, 1.0))
        .expect("ink")
        .2
        + 1;
    let solidest = |columns: std::ops::Range<u32>| {
        let width = frame.resolution().width();
        let pixels = frame.bytes().chunks_exact(4).enumerate();
        pixels
            .filter(|(at, _)| columns.contains(&(*at as u32 % width)))
            .map(|(_, pixel)| pixel[3])
            .max()
            .expect("pixels")
    };
    assert_eq!(
        solidest(left..split),
        191,
        "the first word, three quarters in"
    );
    assert_eq!(solidest(split..right + 1), 64, "the second, one quarter");
}

/// A rising piece starts below its line and comes up to it: halfway in, a
/// ten-pixel rise has it five pixels lower down the frame than at rest.
#[test]
fn a_rising_piece_comes_up_from_below() {
    let top = |at| {
        let mut frame = canvas();
        let reveal = Reveal {
            unit: RevealUnit::Line,
            rise: 10.0,
            stagger: 0.0,
            sweep: Sweep {
                at,
                easing: Easing::Linear,
                backwards: false,
            },
        };
        let style = style(28.0, Rgba::WHITE);
        text::draw_revealing(&mut frame, "Ship it", Font::sans(), &style, &reveal);
        bounds(&frame).expect("ink").1
    };
    assert_eq!(top(0.5), top(1.0) + 5);
}
