//! What a compositor is asked to do.

use crate::frame::{Frame, Resolution};
use crate::properties::Properties;

/// One thing to draw, and how to draw it.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer<'a> {
    /// The layer's pixels, straight RGBA. Usually a decoded source frame.
    pub source: &'a Frame,
    /// Where it goes, how big, how solid — already evaluated for this instant.
    /// The compositor animates nothing itself; it draws one moment.
    pub properties: Properties,
}

impl<'a> Layer<'a> {
    /// A layer drawn exactly as it arrived.
    pub fn plain(source: &'a Frame) -> Self {
        Self {
            source,
            properties: Properties::default(),
        }
    }
}

/// Produces one output frame from the layers visible at one instant.
///
/// The trait exists so a GPU backend can slot in behind it unchanged, with the
/// golden renders proving the two agree. It takes `&mut self` because a
/// backend legitimately owns scratch buffers — reusing one is the difference
/// between a few megabytes and a few hundred megabytes a second.
pub trait Compositor {
    /// Draws `layers` onto `canvas`, **first at the bottom**, matching the order
    /// video tracks appear in a project.
    ///
    /// The canvas is cleared to opaque black first. The output of a render is a
    /// picture, and where nothing covers it, a picture is black — not
    /// transparent, and not whatever the previous frame left behind.
    fn composite(&mut self, canvas: &mut Frame, layers: &[Layer<'_>])
    -> Result<(), CompositeError>;
}

/// Why a frame could not be composited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CompositeError {
    #[error("a {resolution} canvas does not match its {bytes} bytes of buffer")]
    BadCanvas {
        resolution: Resolution,
        bytes: usize,
    },
    #[error("a {resolution} layer does not match its {bytes} bytes of buffer")]
    BadLayer {
        resolution: Resolution,
        bytes: usize,
    },
}
