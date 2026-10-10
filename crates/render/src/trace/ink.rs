//! A pen line as strokes a hand would draw.
//!
//! A traced region is a filled shape, and a dark outline traced is a filled
//! *ring*: drawing its border on with a dash would draw two thin lines, one
//! each side of the outline, never the one line a pen made. So an ink region
//! is cut into **strands**, one per boundary it has — its outside edge, and the
//! edge of every hole in it (each colour the outline encloses). Every ink pixel
//! joins the strand whose boundary is nearest, and the strand records how deep
//! its pixels go.
//!
//! A strand is drawn as one wide stroke along its boundary, as wide as the
//! strand is deep on either side, clipped to the strand's own pixels: as the
//! dash runs along the boundary it uncovers the line beside it, and when it is
//! done every pixel of the strand is uncovered. One edge drawn, one line
//! appearing — the outline is drawn with a single pass of the pen.

use super::outline::Mask;
use super::regions::Region;

/// One boundary of an ink region and the ink nearest it.
pub(super) struct Strand {
    /// The boundary, as a filled shape whose outline it is: the region with its
    /// holes filled for the outside edge, the hole itself for a hole's.
    pub(super) edge: Mask,
    /// The ink pixels nearer this boundary than any other.
    pub(super) reach: Mask,
    /// How far, in pixels, the deepest of them is from the boundary.
    pub(super) depth: u32,
    /// The top of the boundary, for the order strands are drawn in.
    pub(super) top: usize,
    /// Its left.
    pub(super) left: usize,
}

/// A cell of the mask that is no part of the ink, and not yet in a hole.
const UNSET: u32 = u32::MAX;

/// The region's strands, the outside edge first and then its holes from top to
/// bottom. `width` is the picture's.
pub(super) fn strands(region: &Region, width: usize) -> Vec<Strand> {
    let mut ink = Mask::around(region.bounds);
    for &pixel in &region.pixels {
        let pixel = pixel as usize;
        let at = ink.at(pixel % width, pixel / width);
        ink.bits[at] = true;
    }
    let (sides, count) = sides(&ink);
    let (nearest, depth) = nearest(&ink, &sides);
    let mut strands: Vec<Strand> = (0..count)
        .map(|side| {
            let mut edge = blank(&ink);
            let mut reach = blank(&ink);
            for at in 0..ink.bits.len() {
                edge.bits[at] = if side == 0 {
                    sides[at] != 0
                } else {
                    sides[at] == side
                };
                reach.bits[at] = ink.bits[at] && nearest[at] == side;
            }
            let deepest = (0..ink.bits.len())
                .filter(|&at| reach.bits[at])
                .map(|at| depth[at])
                .max()
                .unwrap_or(0);
            overlap(&mut reach, &ink);
            let first = edge.bits.iter().position(|&bit| bit).unwrap_or(0);
            let (column, row) = (first % ink.width, first / ink.width);
            Strand {
                top: usize::try_from(ink.top + isize::try_from(row).unwrap_or(0)).unwrap_or(0),
                left: usize::try_from(ink.left + isize::try_from(column).unwrap_or(0)).unwrap_or(0),
                edge,
                reach,
                depth: deepest,
            }
        })
        .filter(|strand| strand.reach.bits.iter().any(|&bit| bit))
        .collect();
    if strands.len() > 1 {
        strands[1..].sort_by_key(|strand| (strand.top, strand.left));
    }
    strands
}

/// Which side of the ink each cell that is not ink is on: 0 for outside (the
/// margin is outside), 1 and up for each hole. Ink cells are [`UNSET`]. Also
/// answers how many sides there are.
fn sides(ink: &Mask) -> (Vec<u32>, u32) {
    let mut sides = vec![UNSET; ink.bits.len()];
    let mut count = 0;
    for start in 0..ink.bits.len() {
        if ink.bits[start] || sides[start] != UNSET {
            continue;
        }
        sides[start] = count;
        let mut stack = vec![start];
        while let Some(at) = stack.pop() {
            for next in beside(ink, at) {
                if !ink.bits[next] && sides[next] == UNSET {
                    sides[next] = count;
                    stack.push(next);
                }
            }
        }
        count += 1;
    }
    (sides, count)
}

/// For every ink cell, the side whose boundary is nearest and how many steps
/// away it is — a breadth-first walk inward from every boundary at once.
fn nearest(ink: &Mask, sides: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let mut nearest = vec![UNSET; ink.bits.len()];
    let mut depth = vec![0; ink.bits.len()];
    let mut frontier = Vec::new();
    for (at, side) in nearest.iter_mut().enumerate() {
        if !ink.bits[at] {
            continue;
        }
        if let Some(outside) = beside(ink, at).find(|&next| !ink.bits[next]) {
            *side = sides[outside];
            frontier.push(at);
        }
    }
    let mut step = 0;
    while !frontier.is_empty() {
        step += 1;
        let mut next_frontier = Vec::new();
        for at in frontier {
            for next in beside(ink, at) {
                if ink.bits[next] && nearest[next] == UNSET {
                    nearest[next] = nearest[at];
                    depth[next] = step;
                    next_frontier.push(next);
                }
            }
        }
        frontier = next_frontier;
    }
    (nearest, depth)
}

/// Grows a strand's reach a pixel into the ink around it. Two strands meet
/// inside a line, and each clip is traced on its own, so without the overlap
/// the seam between them shows as a hairline of whatever is underneath.
fn overlap(reach: &mut Mask, ink: &Mask) {
    let grown: Vec<usize> = (0..reach.bits.len())
        .filter(|&at| ink.bits[at] && !reach.bits[at])
        .filter(|&at| beside(ink, at).any(|next| reach.bits[next]))
        .collect();
    for at in grown {
        reach.bits[at] = true;
    }
}

/// An empty mask over the same box as `like`.
fn blank(like: &Mask) -> Mask {
    Mask {
        left: like.left,
        top: like.top,
        width: like.width,
        height: like.height,
        bits: vec![false; like.bits.len()],
    }
}

/// The cells up, down, left and right of `at` inside the mask.
fn beside(mask: &Mask, at: usize) -> impl Iterator<Item = usize> {
    let (x, y, width) = (at % mask.width, at / mask.width, mask.width);
    [
        (x > 0).then(|| at - 1),
        (x + 1 < width).then(|| at + 1),
        (y > 0).then(|| at - width),
        (y + 1 < mask.height).then(|| at + width),
    ]
    .into_iter()
    .flatten()
}
