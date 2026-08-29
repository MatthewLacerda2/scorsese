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
