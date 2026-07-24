//! The raw frame buffer that travels between decode, compositing, and encode.

use crate::settings::Resolution;

/// Bytes per pixel in the interchange format.
pub const BYTES_PER_PIXEL: usize = 4;

/// The pixel format frames are carried in, as ffmpeg names it.
///
/// RGBA, 8 bits a channel: the compositor blends with alpha and reads
/// individual channels, which is awkward in planar YUV and trivial here. The
/// cost is bandwidth — a 1080p frame is 8 MB — and it is worth it, because the
/// alternative is doing colour maths in the encoder's format. Converting to
/// YUV is the encoder's job, at the very end, once.
pub const PIXEL_FORMAT: &str = "rgba";

/// One uncompressed frame.
///
/// Allocated once per segment and reused for every frame in it: at 1080p30
/// that is the difference between 8 MB of allocation and 250 MB a second.
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

    pub const fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// How many bytes one frame of this size occupies.
    pub const fn byte_count(&self) -> usize {
        self.pixels.len()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.pixels
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Resets the frame to opaque black — what a gap in the timeline looks
    /// like, and what a clip whose source ran out falls back to.
    pub fn fill_black(&mut self) {
        for pixel in self.pixels.chunks_exact_mut(BYTES_PER_PIXEL) {
            pixel.copy_from_slice(&[0, 0, 0, u8::MAX]);
        }
    }
}
