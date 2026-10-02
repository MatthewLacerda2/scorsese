//! How big the thing a layer draws is, as opposed to the raster it is drawn
//! on.
//!
//! A shape, a title or an icon is drawn on a raster the size of the frame, so
//! the raster's own size says nothing about how much light the layer has to
//! give off. Its pixels do: the box round everything with any alpha at all.

use crate::frame::{BYTES_PER_PIXEL, Resolution};

/// The longer side of the box round every pixel of `source` with any alpha,
/// in pixels — zero when there is none.
///
/// A box and not an area, because a halo's reach is a distance: a long thin
/// line is as long as its box whatever its width, and a glow along it reaches
/// as far as its length allows.
pub(super) fn longest_side(source: &[u8], resolution: Resolution) -> usize {
    let width = resolution.width() as usize;
    let mut across = (usize::MAX, 0);
    let mut down = (usize::MAX, 0);
    for (y, row) in source.chunks_exact(width * BYTES_PER_PIXEL).enumerate() {
        let solid = |pixel: &[u8]| pixel[3] != 0;
        let Some(first) = row.chunks_exact(BYTES_PER_PIXEL).position(solid) else {
            continue;
        };
        let last = row
            .chunks_exact(BYTES_PER_PIXEL)
            .rposition(solid)
            .unwrap_or(first);
        across = (across.0.min(first), across.1.max(last));
        down = (down.0.min(y), y);
    }
    if across.0 == usize::MAX {
        return 0;
    }
    (across.1 - across.0 + 1).max(down.1 - down.0 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `width` × `height` raster, transparent but for the pixels named.
    fn raster(width: u32, height: u32, solid: &[(usize, usize)]) -> (Vec<u8>, Resolution) {
        let resolution = Resolution::new(width, height).expect("a legal raster");
        let mut bytes = vec![0; resolution.pixels() * BYTES_PER_PIXEL];
        for &(x, y) in solid {
            bytes[(y * width as usize + x) * BYTES_PER_PIXEL + 3] = 1;
        }
        (bytes, resolution)
    }

    #[test]
    fn the_box_round_what_is_drawn_and_not_the_raster() {
        let (bytes, resolution) = raster(16, 8, &[(3, 2), (9, 4), (5, 5)]);
        assert_eq!(
            longest_side(&bytes, resolution),
            7,
            "x 3 to 9 is seven wide"
        );

        let (bytes, resolution) = raster(16, 8, &[(4, 0), (4, 7)]);
        assert_eq!(
            longest_side(&bytes, resolution),
            8,
            "and y 0 to 7 eight tall"
        );

        let (bytes, resolution) = raster(16, 8, &[(15, 7)]);
        assert_eq!(longest_side(&bytes, resolution), 1, "one pixel is one wide");
    }

    #[test]
    fn nothing_drawn_is_no_size_at_all() {
        let (bytes, resolution) = raster(16, 8, &[]);
        assert_eq!(longest_side(&bytes, resolution), 0);
    }
}
