//! What two frames of a glance are compared by, and a blank one told by.

use scorsese_compositor::Frame;

/// Cells across and down a [`Signature`].
const SIGNATURE_SIDE: usize = 16;

/// A frame whose brightest and darkest cells differ by less than this (on
/// 0–255) shows nothing: black, a flat colour, a fade's bottom. The range
/// rather than a spread, so a small caption on black — a few cells lit — is
/// a frame.
const BLANK_RANGE: f64 = 12.0;

/// A frame's brightness averaged over a [`SIGNATURE_SIDE`]-square grid of
/// cells: what two frames are compared by, and a blank one is told by.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Signature(pub(super) [f64; SIGNATURE_SIDE * SIGNATURE_SIDE]);

impl Signature {
    pub(super) fn of(frame: &Frame) -> Self {
        let width = frame.resolution().width() as usize;
        let height = frame.resolution().height() as usize;
        let mut sums = [0.0; SIGNATURE_SIDE * SIGNATURE_SIDE];
        let mut counts = [0_u32; SIGNATURE_SIDE * SIGNATURE_SIDE];
        for (pixel, rgba) in frame.bytes().chunks_exact(4).enumerate() {
            let (x, y) = (pixel % width, pixel / width);
            let cell = (y * SIGNATURE_SIDE / height) * SIGNATURE_SIDE + x * SIGNATURE_SIDE / width;
            sums[cell] += 0.299 * f64::from(rgba[0])
                + 0.587 * f64::from(rgba[1])
                + 0.114 * f64::from(rgba[2]);
            counts[cell] += 1;
        }
        for (sum, count) in sums.iter_mut().zip(counts) {
            *sum /= f64::from(count.max(1));
        }
        Self(sums)
    }

    /// Whether it shows nothing: its cells' brightness hardly ranges.
    pub(super) fn blank(&self) -> bool {
        let (low, high) = self
            .0
            .iter()
            .fold((f64::MAX, f64::MIN), |(low, high), cell| {
                (low.min(*cell), high.max(*cell))
            });
        high - low < BLANK_RANGE
    }

    /// How far apart two frames are: their cells' mean absolute difference.
    pub(super) fn difference(&self, other: &Self) -> f64 {
        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f64>()
            / self.0.len() as f64
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::contact::glance) fn flat(level: f64) -> Signature {
        Signature([level; SIGNATURE_SIDE * SIGNATURE_SIDE])
    }

    /// A busy signature, one of its own for each `which`.
    pub(in crate::contact::glance) fn busy(which: usize) -> Signature {
        let mut cells = [0.0; SIGNATURE_SIDE * SIGNATURE_SIDE];
        for (index, cell) in cells.iter_mut().enumerate() {
            *cell = 30.0 + ((index * 7 + which * 37) % 190) as f64;
        }
        Signature(cells)
    }

    #[test]
    fn a_flat_frame_is_blank_and_a_busy_one_is_not() {
        assert!(flat(0.0).blank());
        assert!(flat(200.0).blank());
        assert!(!busy(0).blank());
    }

    /// A small caption on black is not nothing: three cells a third lit.
    #[test]
    fn a_small_caption_on_black_is_not_blank() {
        let mut caption = flat(0.0);
        for cell in &mut caption.0[120..123] {
            *cell = 30.0;
        }
        assert!(!caption.blank());
        let mut noise = flat(0.0);
        noise.0[7] = 3.0;
        assert!(noise.blank(), "codec noise on black is still black");
    }
}
