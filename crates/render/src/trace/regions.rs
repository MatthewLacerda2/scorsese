//! A picture of a few colours, cut into the regions a tracer draws.
//!
//! A region is a run of touching pixels of one palette colour — the face, the
//! sash, one eye. Every pixel belongs to one region or to none (transparent, or
//! the dropped background), and no two regions overlap: a region is exactly
//! what shows of it, holes and all. That is what lets the regions be drawn in
//! any order a hand would draw them, rather than in the back-to-front order an
//! overlapping stack would need.

/// A pixel that is in no region: transparent in the picture, or the background
/// that was dropped.
pub(super) const NONE: u8 = u8::MAX;

/// The picture as palette indices, a byte a pixel, [`NONE`] where nothing is.
pub(super) struct Indexed {
    /// Pixels across.
    pub(super) width: usize,
    /// Pixels down.
    pub(super) height: usize,
    /// One palette index a pixel, row by row.
    pub(super) cells: Vec<u8>,
}

/// One region: its colour, and where its pixels are.
pub(super) struct Region {
    /// Its palette index.
    pub(super) colour: u8,
    /// Its pixels, as indices into [`Indexed::cells`].
    pub(super) pixels: Vec<u32>,
    /// Its bounding box: left, top, right and bottom, the last two exclusive.
    pub(super) bounds: [usize; 4],
}

impl Indexed {
    /// Every region, each pixel in exactly one, in no particular order.
    pub(super) fn regions(&self) -> Vec<Region> {
        let mut seen = vec![false; self.cells.len()];
        let mut regions = Vec::new();
        for start in 0..self.cells.len() {
            if seen[start] || self.cells[start] == NONE {
                continue;
            }
            regions.push(self.flood(start, &mut seen));
        }
        regions
    }

    /// Paints every region smaller than `smallest` pixels the colour it shares
    /// the most edge with — a speck of anti-aliasing becomes part of the line
    /// or the fill it sits on, rather than a shape of its own. Twice, because
    /// two specks side by side can each have only the other to join.
    pub(super) fn absorb_specks(&mut self, smallest: usize) {
        for _ in 0..2 {
            for region in self.regions() {
                if region.pixels.len() >= smallest {
                    continue;
                }
                if let Some(colour) = self.commonest_neighbour(&region) {
                    for &pixel in &region.pixels {
                        self.cells[pixel as usize] = colour;
                    }
                }
            }
        }
    }

    /// Drops the background: the colour most of the picture's edge is, wherever
    /// it reaches from the edge. The same colour inside the drawing (the white
    /// of an eye) stays, because nothing from the edge reaches it.
    pub(super) fn drop_background(&mut self) {
        let edge = self.edge();
        let mut counts = [0usize; 256];
        for &pixel in &edge {
            counts[usize::from(self.cells[pixel])] += 1;
        }
        counts[usize::from(NONE)] = 0;
        let Some((background, _)) = counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count > 0)
            .max_by_key(|(_, count)| **count)
        else {
            return;
        };
        let mut seen = vec![false; self.cells.len()];
        for pixel in edge {
            if usize::from(self.cells[pixel]) == background && !seen[pixel] {
                for inside in self.flood(pixel, &mut seen).pixels {
                    self.cells[inside as usize] = NONE;
                }
            }
        }
    }

    /// The region `start` is in, marking its pixels seen.
    fn flood(&self, start: usize, seen: &mut [bool]) -> Region {
        let colour = self.cells[start];
        let mut region = Region {
            colour,
            pixels: Vec::new(),
            bounds: [usize::MAX, usize::MAX, 0, 0],
        };
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(pixel) = stack.pop() {
            let (x, y) = (pixel % self.width, pixel / self.width);
            region.pixels.push(u32::try_from(pixel).unwrap_or(u32::MAX));
            let bounds = &mut region.bounds;
            *bounds = [
                bounds[0].min(x),
                bounds[1].min(y),
                bounds[2].max(x + 1),
                bounds[3].max(y + 1),
            ];
            for next in self.beside(pixel) {
                if !seen[next] && self.cells[next] == colour {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        region
    }

    /// The colour of a region's neighbours that it touches most, if it touches
    /// any colour at all.
    fn commonest_neighbour(&self, region: &Region) -> Option<u8> {
        let mut counts = [0usize; 256];
        for &pixel in &region.pixels {
            for next in self.beside(pixel as usize) {
                let colour = self.cells[next];
                if colour != region.colour {
                    counts[usize::from(colour)] += 1;
                }
            }
        }
        counts[usize::from(NONE)] = 0;
        let (colour, count) = counts
            .iter()
            .enumerate()
            .max_by_key(|(colour, count)| (**count, std::cmp::Reverse(*colour)))?;
        (*count > 0).then(|| u8::try_from(colour).unwrap_or(NONE))
    }

    /// The pixels up, down, left and right of `pixel` that are in the picture.
    pub(super) fn beside(&self, pixel: usize) -> impl Iterator<Item = usize> {
        let (x, y, width) = (pixel % self.width, pixel / self.width, self.width);
        [
            (x > 0).then(|| pixel - 1),
            (x + 1 < width).then(|| pixel + 1),
            (y > 0).then(|| pixel - width),
            (y + 1 < self.height).then(|| pixel + width),
        ]
        .into_iter()
        .flatten()
    }

    /// The pixels along the picture's four edges.
    fn edge(&self) -> Vec<usize> {
        let (width, height) = (self.width, self.height);
        let rows = (0..width).flat_map(|x| [x, (height - 1) * width + x]);
        let columns = (0..height).flat_map(|y| [y * width, y * width + width - 1]);
        rows.chain(columns).collect()
    }
}
