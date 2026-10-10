//! An ink line cut into one stroke per edge (`ink`).

use crate::trace::ink::{Strand, strands};
use crate::trace::regions::Region;

const WIDTH: usize = 9;
const HEIGHT: usize = 14;

/// Whether the picture's pixel `(x, y)` is in a hole of the ladder: two
/// windows, one above the other, in a 9×14 block of ink whose every band is
/// at least two pixels thick and the bar between them four.
fn hole(x: usize, y: usize) -> bool {
    (2..7).contains(&x) && ((2..5).contains(&y) || (9..12).contains(&y))
}

fn ladder() -> Vec<Strand> {
    let pixels = (0..WIDTH * HEIGHT)
        .filter(|&at| !hole(at % WIDTH, at / WIDTH))
        .map(|at| at as u32)
        .collect();
    let region = Region {
        colour: 0,
        pixels,
        bounds: [0, 0, WIDTH, HEIGHT],
    };
    strands(&region, WIDTH)
}

/// Whether the strand's `mask` holds the picture's pixel `(x, y)`.
fn holds(strand: &Strand, edge: bool, x: usize, y: usize) -> bool {
    let mask = if edge { &strand.edge } else { &strand.reach };
    mask.bits[mask.at(x, y)]
}

#[test]
fn the_outside_comes_first_then_each_hole_from_the_top() {
    let strands = ladder();
    assert_eq!(strands.len(), 3);
    // The outside's edge is the whole shape, holes filled.
    assert!(holds(&strands[0], true, 0, 0) && holds(&strands[0], true, 4, 3));
    // A hole's edge is that hole and nothing else.
    assert!(holds(&strands[1], true, 4, 3) && !holds(&strands[1], true, 4, 10));
    assert!(holds(&strands[2], true, 4, 10) && !holds(&strands[2], true, 4, 3));
    assert!(!holds(&strands[1], true, 4, 6) && !holds(&strands[1], true, 0, 0));
}

#[test]
fn every_pixel_of_ink_is_reached_by_the_edge_nearest_it() {
    let strands = ladder();
    // The bar between the windows: its top row is the upper window's, its
    // bottom row the lower's, and neither reaches across it.
    assert!(holds(&strands[1], false, 4, 5) && !holds(&strands[2], false, 4, 5));
    assert!(holds(&strands[2], false, 4, 8) && !holds(&strands[1], false, 4, 8));
    // A corner is the outside's alone.
    assert!(holds(&strands[0], false, 0, 0) && !holds(&strands[1], false, 0, 0));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let reached = strands.iter().any(|strand| holds(strand, false, x, y));
            // Every pixel of ink is drawn, and no reach strays off the ink.
            assert_eq!(reached, !hole(x, y), "({x}, {y})");
        }
    }
}

#[test]
fn a_strand_is_as_deep_as_the_ink_beside_its_edge() {
    let depths: Vec<u32> = ladder().iter().map(|strand| strand.depth).collect();
    assert_eq!(depths, [1, 1, 1]);
}

#[test]
fn neighbouring_strands_overlap_by_a_pixel() {
    let strands = ladder();
    // Rows 6 and 7 of the bar are where the windows' reaches meet.
    assert!(holds(&strands[1], false, 4, 7) && holds(&strands[2], false, 4, 6));
}
