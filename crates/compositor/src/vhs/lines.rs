//! The two things a tape does to whole rows: scanlines, and the head switch.
//!
//! Both are geometry down the frame and nothing else — neither reads a pixel —
//! so both are worked out once per row and handed to the loop that does.

/// How dark a scanline goes at `scanlines` of `1.0`: most of the way, and not
/// all of the way.
///
/// Black lines would make the picture half a picture; what a line pitch reads
/// as is a texture over the whole image, and it stops reading as one the moment
/// the dark rows carry nothing.
const DARKEST: f64 = 0.6;

/// How many line pairs a tape's picture is divided into, top to bottom.
///
/// **A count and not a pixel pitch**, for the reason a grain cell is measured
/// against a reference height: the same number has to read as the same texture
/// whether the edit is delivered at 480 lines or at 2160, and a pattern of
/// every-other-pixel is invisible at the second. This is roughly what a tape
/// actually held, so a 480-tall delivery comes out at one dark row per light
/// one — which is the picture this is imitating.
const PAIRS: u32 = 240;

/// The tallest a head-switching band gets, as a fraction of the layer's height.
///
/// A tape's is a handful of lines out of hundreds; this is an order more than
/// that, because `1.0` is the top of a range somebody turns rather than a
/// measurement of any format.
const TALLEST: f64 = 0.08;

/// The scanline pattern of one layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Lines {
    /// How many rows one light-and-dark pair spans. Never below two, because
    /// there is nothing finer for a pattern of two states to be.
    period: usize,
    /// How much of its brightness a dark row keeps.
    keeps: f64,
}

impl Lines {
    pub(crate) fn new(scanlines: f64, height: u32) -> Self {
        Self {
            period: ((height / PAIRS) as usize).max(2),
            keeps: 1.0 - scanlines.clamp(0.0, 1.0) * DARKEST,
        }
    }

    /// What this row keeps of its brightness: all of it on a light row, less on
    /// a dark one.
    ///
    /// The first half of every period is the dark one. Which half hardly
    /// matters to the picture and matters entirely to a reference frame, so it
    /// is stated rather than left to whichever way the comparison happened to
    /// be written.
    pub(crate) fn fall(&self, y: usize) -> f64 {
        if y % self.period < self.period / 2 {
            self.keeps
        } else {
            1.0
        }
    }
}

/// Where the heads hand over, as a band of rows at the bottom of the layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Band {
    /// The first row of the band. Equal to the height when there is no band.
    top: usize,
    /// How many rows it spans. Zero when there is none.
    rows: usize,
}

impl Band {
    /// The band `head_switch` asks for on a layer this tall.
    ///
    /// **Under half a row is no band at all**, which is the same threshold
    /// [`crate::blur::radius`] applies and for the same reason: a knob barely
    /// off its stop should cost nothing rather than tear one row by a pixel.
    pub(crate) fn new(head_switch: f64, height: u32) -> Self {
        let height = height as usize;
        let rows = head_switch.clamp(0.0, 1.0) * TALLEST * height as f64;
        let rows = if rows.is_nan() || rows < 0.5 {
            0
        } else {
            (rows.round() as usize).min(height)
        };
        Self {
            top: height - rows,
            rows,
        }
    }

    /// How deep into the band this row sits — `None` above it, and otherwise
    /// running up to `1.0` on the very last row of the picture.
    ///
    /// **Growing downward**, so the tear widens toward the bottom edge rather
    /// than displacing the whole band by one amount. That is what the artefact
    /// looks like: the error is worst where the switch is, and the switch is at
    /// the bottom.
    pub(crate) fn depth(&self, y: usize) -> Option<f64> {
        if self.rows == 0 || y < self.top {
            return None;
        }
        Some((y - self.top + 1) as f64 / self.rows as f64)
    }
}

/// The row arithmetic, named where it is decided. `tests/taping/` asserts what
/// a darkened row and a torn band *look* like; these are the two numbers that
/// decide which rows they are.
#[cfg(test)]
mod tests {
    use super::*;

    /// A count of line pairs down the picture and never a pitch in pixels, so
    /// the texture is the same at every delivery size — and never finer than
    /// two rows, because a pattern of two states has nothing finer to be.
    #[test]
    fn the_pitch_is_a_count_and_bottoms_out_at_two_rows() {
        assert_eq!(Lines::new(0.5, 1080).period, 4);
        assert_eq!(Lines::new(0.5, 480).period, 2);
        assert_eq!(Lines::new(0.5, 64).period, 2, "and never below it");
    }

    /// The first half of a period is the dark one. Which half is invisible to
    /// anyone reasoning about it and decides every reference frame, so it is
    /// asserted rather than left to whichever way the comparison was written.
    #[test]
    fn the_first_half_of_a_period_is_the_dark_half() {
        let lines = Lines::new(1.0, 1080);
        assert_eq!((lines.fall(0), lines.fall(1)), (0.4, 0.4));
        assert_eq!((lines.fall(2), lines.fall(3)), (1.0, 1.0));
        assert_eq!(lines.fall(4), 0.4, "and the pattern repeats");
    }

    /// Most of the way to black at `1.0`, and clamped there — a track
    /// overshooting past the top of the range must not invert a row.
    #[test]
    fn a_dark_row_keeps_a_fixed_share_of_its_brightness() {
        assert_eq!(Lines::new(0.0, 1080).fall(0), 1.0);
        assert_eq!(Lines::new(0.5, 1080).fall(0), 0.7);
        assert_eq!(Lines::new(4.0, 1080).fall(0), 0.4);
        assert_eq!(Lines::new(-1.0, 1080).fall(0), 1.0);
    }

    /// A twelfth of the height at `1.0`, measured from the bottom.
    #[test]
    fn the_band_is_a_fraction_of_the_height_at_the_bottom() {
        let band = Band::new(1.0, 1000);
        assert_eq!((band.top, band.rows), (920, 80));
        assert_eq!(band.depth(919), None, "the row above it is untouched");
        assert_eq!(band.depth(920), Some(1.0 / 80.0));
        assert_eq!(band.depth(999), Some(1.0), "and the last row is the worst");
    }

    /// **Under half a row is no band**, so a knob barely off its stop costs
    /// nothing rather than tearing one row by a pixel.
    #[test]
    fn nothing_worth_tearing_is_no_band_at_all() {
        for asked in [0.0, -1.0, f64::NAN, 0.006] {
            let band = Band::new(asked, 1000);
            assert_eq!(band.rows, 0, "{asked}");
            assert_eq!(band.depth(999), None, "{asked}");
        }
        // 0.006 × 0.08 × 1000 is 0.48, under the bar; 0.007 is 0.56, over it.
        assert_eq!(Band::new(0.007, 1000).rows, 1);
    }
}
