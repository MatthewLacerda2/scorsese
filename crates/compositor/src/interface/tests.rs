use scorsese_core::Rgba;

use super::{disc, fade, line_width, outlined, over, ring, rounded, symbol};
use crate::area::Area;
use crate::frame::{Frame, Resolution};
use crate::text::Font;

const WHITE: Rgba = Rgba {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

fn clear(width: u32, height: u32) -> Frame {
    let mut frame = Frame::black(Resolution::new(width, height).expect("a raster"));
    frame.fill_transparent();
    frame
}

fn alpha(frame: &Frame, x: u32, y: u32) -> u8 {
    let width = frame.resolution().width();
    frame.bytes()[((y * width + x) * 4 + 3) as usize]
}

#[test]
fn a_rounded_box_fills_its_middle_and_not_its_corner() {
    let mut frame = clear(100, 100);
    let area = Area {
        left: 10.0,
        top: 10.0,
        width: 80.0,
        height: 40.0,
    };
    rounded(&mut frame, area, 20.0, WHITE);
    assert_eq!(alpha(&frame, 50, 30), 255);
    assert_eq!(alpha(&frame, 11, 11), 0, "the corner is cut");
    assert_eq!(alpha(&frame, 50, 70), 0, "nothing below it");
}

#[test]
fn a_ring_is_hollow_and_a_disc_is_not() {
    let mut frame = clear(100, 100);
    ring(&mut frame, (30.0, 50.0), 20.0, (WHITE, 4.0));
    disc(&mut frame, (75.0, 50.0), 10.0, WHITE);
    assert_eq!(alpha(&frame, 30, 50), 0, "a ring's middle is empty");
    assert!(alpha(&frame, 50, 50) > 200, "its line is drawn");
    assert_eq!(alpha(&frame, 75, 50), 255);
}

#[test]
fn a_symbol_stays_in_its_square() {
    let mut frame = clear(100, 100);
    let heart = crate::icon::find("heart").expect("lucide has one");
    symbol(&mut frame, heart, (40.0, 40.0), 24.0, (WHITE, 2.0));
    let drawn: Vec<(u32, u32)> = (0..100)
        .flat_map(|y| (0..100).map(move |x| (x, y)))
        .filter(|&(x, y)| alpha(&frame, x, y) > 0)
        .collect();
    assert!(!drawn.is_empty());
    assert!(
        drawn
            .iter()
            .all(|&(x, y)| (39..=64).contains(&x) && (39..=64).contains(&y))
    );
}

#[test]
fn a_fade_runs_from_one_colour_to_the_other_over_its_rows() {
    let mut frame = clear(4, 10);
    let from = Rgba { a: 0, ..WHITE };
    fade(&mut frame, 0.0, 10.0, (from, WHITE));
    assert!(alpha(&frame, 0, 0) < 20);
    assert!(alpha(&frame, 0, 9) > 235);
    assert!(alpha(&frame, 0, 3) < alpha(&frame, 0, 6));
}

#[test]
fn a_layer_is_laid_over_only_where_it_has_ink_and_only_at_its_size() {
    let mut frame = Frame::black(Resolution::new(4, 4).expect("a raster"));
    let mut layer = clear(4, 4);
    rounded(
        &mut layer,
        Area {
            left: 0.0,
            top: 0.0,
            width: 2.0,
            height: 4.0,
        },
        0.0,
        WHITE,
    );
    over(&mut frame, &layer);
    assert_eq!(&frame.bytes()[0..3], &[255, 255, 255]);
    assert_eq!(
        &frame.bytes()[12..15],
        &[0, 0, 0],
        "the bare half is untouched"
    );
    let mut other = Frame::black(Resolution::new(2, 2).expect("a raster"));
    over(&mut other, &layer);
    assert!(other.bytes().chunks(4).all(|pixel| pixel[..3] == [0, 0, 0]));
}

#[test]
fn a_longer_line_sets_wider() {
    let font = Font::sans();
    assert!(line_width(font, "Follow", 20.0) > line_width(font, "Go", 20.0));
}

#[test]
fn a_fade_starts_at_its_top_mixes_between_its_colours_and_draws_nothing_with_no_height() {
    let mut frame = clear(2, 20);
    let from = Rgba {
        r: 100,
        g: 100,
        b: 100,
        a: 255,
    };
    let to = Rgba {
        r: 200,
        g: 200,
        b: 200,
        a: 255,
    };
    fade(&mut frame, 5.0, 10.0, (from, to));
    let red = |y: u32| frame.bytes()[(y * 8) as usize];
    assert_eq!(alpha(&frame, 0, 4), 0, "nothing above its top");
    assert!(red(5) < 110 && red(14) > 190, "{} … {}", red(5), red(14));
    assert!((140..=160).contains(&red(9)) || (140..=160).contains(&red(10)));
    let mut opaque = Frame::black(Resolution::new(2, 4).expect("a raster"));
    fade(&mut opaque, 0.5, 0.0, (WHITE, WHITE));
    assert_eq!(alpha(&opaque, 0, 0), 255, "no rows for no height");
}

#[test]
fn an_outline_draws_the_edge_not_the_middle_and_a_radius_is_held_to_the_box() {
    let mut frame = clear(100, 100);
    let area = Area {
        left: 10.0,
        top: 10.0,
        width: 80.0,
        height: 40.0,
    };
    outlined(&mut frame, area, 8.0, (WHITE, 4.0));
    assert!(alpha(&frame, 50, 10) > 200, "the top edge");
    assert_eq!(alpha(&frame, 50, 30), 0, "an outline's middle is empty");
    let mut pill = clear(100, 100);
    rounded(&mut pill, area, 500.0, WHITE);
    assert_eq!(
        alpha(&pill, 50, 30),
        255,
        "a radius past half the height makes a pill"
    );
    assert_eq!(alpha(&pill, 13, 30), 255, "its end is round, not pinched");
    assert_eq!(alpha(&pill, 11, 12), 0, "and its corner is cut");
}
