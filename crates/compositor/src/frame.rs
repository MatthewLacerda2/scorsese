//! The raster everything is drawn into, and the buffer that carries it.
//!
//! This lives here rather than in `scorsese-render` because a frame buffer is
//! the compositor's currency: the renderer moves frames between ffmpeg and
//! here, but this crate is what fills them. `scorsese-render` re-exports both
//! types, so a caller need not care which crate defines them.

use std::fmt;
use std::str::FromStr;

use scorsese_core::Rgba;

/// Bytes per pixel in the interchange format.
pub const BYTES_PER_PIXEL: usize = 4;

/// The pixel format frames are carried in, as ffmpeg names it.
///
/// RGBA, 8 bits a channel, **straight** alpha — not premultiplied. Straight
/// because that is what ffmpeg decodes to and what a PNG stores, so the format
/// needs no conversion at either edge of the pipeline. The CPU compositor
/// premultiplies internally, where its rasteriser wants that, and converts
/// back; nothing outside it has to know.
pub const PIXEL_FORMAT: &str = "rgba";

/// The pixel dimensions of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    width: u32,
    height: u32,
}

impl Resolution {
    #[expect(
        dead_code,
        reason = "1920x1080 named once for whoever needs a default in code; the \
                  CLI and MCP server currently spell theirs as text instead"
    )]
    pub(crate) const HD: Self = Self {
        width: 1920,
        height: 1080,
    };

    /// Both dimensions must be non-zero and even.
    ///
    /// Even because the encoder writes 4:2:0 chroma, where an odd dimension has
    /// no defined half. Refusing here, before anything spawns, beats an ffmpeg
    /// error surfacing from inside a render that already started.
    pub fn new(width: u32, height: u32) -> Result<Self, ResolutionError> {
        if width == 0 || height == 0 {
            return Err(ResolutionError::Zero);
        }
        if width % 2 == 1 || height % 2 == 1 {
            return Err(ResolutionError::Odd { width, height });
        }
        Ok(Self { width, height })
    }

    /// The size of a raster that is **not** encoded — a decoded source frame
    /// resting on the canvas at its own size.
    ///
    /// Only the zero check applies here. The even rule belongs to the encoder:
    /// 4:2:0 chroma has no defined half of an odd dimension, which is a fact
    /// about the file being written rather than about every buffer in the
    /// process. A 33×33 logo is a perfectly good layer, and refusing it would
    /// mean refusing to render a project over the shape of one of its sources.
    pub fn source(width: u32, height: u32) -> Result<Self, ResolutionError> {
        if width == 0 || height == 0 {
            return Err(ResolutionError::Zero);
        }
        Ok(Self { width, height })
    }

    /// Pixels across — even, unless this came from [`Resolution::source`].
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Pixels down, under the same rule as [`Resolution::width`].
    pub const fn height(self) -> u32 {
        self.height
    }

    pub(crate) const fn pixels(self) -> usize {
        self.width as usize * self.height as usize
    }

    /// Width over height — derived, never stored, so it cannot disagree with
    /// the dimensions it describes.
    #[expect(
        dead_code,
        reason = "the one derivation of a raster nothing asks for yet; kept \
                  beside the dimensions so it can never be stored separately"
    )]
    pub(crate) fn aspect(self) -> f64 {
        f64::from(self.width) / f64::from(self.height)
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl FromStr for Resolution {
    type Err = ResolutionError;

    /// Parses `1920x1080`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (width, height) = text
            .split_once(['x', 'X'])
            .ok_or(ResolutionError::Malformed)?;
        let parse = |part: &str| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| ResolutionError::Malformed)
        };
        Self::new(parse(width)?, parse(height)?)
    }
}

/// Dimensions that cannot be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResolutionError {
    /// The text was not `WIDTHxHEIGHT` at all — only [`FromStr`] can produce
    /// this one.
    #[error("expected a resolution like `1920x1080`")]
    Malformed,
    /// A dimension of zero, which no buffer, canvas or source has a meaning
    /// for.
    #[error("a resolution cannot have a zero dimension")]
    Zero,
    /// Rejected by [`Resolution::new`] only; [`Resolution::source`] allows it,
    /// since the even rule belongs to the encoder and not to every raster.
    #[error("{width}x{height} has an odd dimension; 4:2:0 video needs both to be even")]
    Odd {
        /// The width asked for.
        width: u32,
        /// The height asked for.
        height: u32,
    },
}

/// One uncompressed frame.
///
/// Allocated once and reused for every frame of a render: at 1080p30 that is
/// the difference between 8 MB of allocation and 250 MB a second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    resolution: Resolution,
    pixels: Vec<u8>,
}

impl Frame {
    /// An opaque black frame.
    pub fn black(resolution: Resolution) -> Self {
        let mut frame = Self {
            resolution,
            pixels: vec![0; resolution.pixels() * BYTES_PER_PIXEL],
        };
        frame.fill_black();
        frame
    }

    /// The size the buffer was laid out for. Fixed at construction, so it can
    /// never drift from the bytes it describes.
    pub const fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// How many bytes one frame of this size occupies.
    pub const fn byte_count(&self) -> usize {
        self.pixels.len()
    }

    /// The pixels themselves, row-major in [`PIXEL_FORMAT`] — the form that
    /// goes to ffmpeg's stdin unchanged.
    pub fn bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// The same buffer to write over, which is how a decoder fills a frame in
    /// place instead of handing back a fresh allocation every time.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Resets the frame to opaque black — what a gap in the timeline looks
    /// like, what a clip whose source ran out falls back to, and what a canvas
    /// starts as before anything is composited onto it.
    pub(crate) fn fill_black(&mut self) {
        self.fill(Rgba::BLACK);
    }

    /// Paints the whole frame one colour, alpha included.
    ///
    /// The flat half of a slug card: a panel of colour is what the prompt is
    /// then drawn over. A translucent colour here is a translucent *layer* —
    /// nothing is blended with what was in the buffer before, because a layer
    /// describes what it contributes and the compositor decides how that meets
    /// the tracks below it.
    pub fn fill(&mut self, color: Rgba) {
        self.fill_rows(0..self.resolution.height(), color);
    }

    /// Paints whole rows one colour and leaves the rest of the frame alone.
    ///
    /// Rows rather than a rectangle, because what needs one is a band across
    /// the picture — a narration card at the foot of the frame — and a general
    /// rectangle would be a clipping model this crate does not otherwise have.
    /// A range reaching past the last row is cut back to it rather than
    /// refused; a band is a fraction of a raster somebody rounded.
    pub(crate) fn fill_rows(&mut self, rows: std::ops::Range<u32>, color: Rgba) {
        let height = self.resolution.height();
        let stride = self.resolution.width() as usize * BYTES_PER_PIXEL;
        let first = rows.start.min(height) as usize * stride;
        let last = rows.end.clamp(rows.start, height) as usize * stride;
        let channels = color.channels();
        for pixel in self.pixels[first..last].chunks_exact_mut(BYTES_PER_PIXEL) {
            pixel.copy_from_slice(&channels);
        }
    }

    /// Resets the frame to nothing at all — transparent everywhere.
    ///
    /// What a layer contributes when it has nothing to show. Distinct from
    /// resetting to black, and the distinction matters: an upper layer that
    /// went opaque black would paint over the tracks below it, where a
    /// transparent one lets them through. Over a black canvas the two look
    /// identical, which is why the difference only shows up once there are
    /// layers.
    pub fn fill_transparent(&mut self) {
        self.pixels.fill(0);
    }

    /// True when every pixel is fully opaque.
    ///
    /// Worth asking, because straight and premultiplied alpha are the *same
    /// bytes* for an opaque pixel — so an opaque frame can go straight to a
    /// rasteriser that wants premultiplied, with no conversion at all. Decoded
    /// video is always opaque, which is the common case.
    pub(crate) fn is_opaque(&self) -> bool {
        self.pixels
            .chunks_exact(BYTES_PER_PIXEL)
            .all(|pixel| pixel[3] == u8::MAX)
    }
}
