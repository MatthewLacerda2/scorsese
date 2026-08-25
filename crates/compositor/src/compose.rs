//! What a compositor is asked to do.

use scorsese_core::{Anchor, Origin};

use crate::frame::{Frame, Resolution};
use crate::properties::Properties;

/// One thing to draw, and how to draw it.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer<'a> {
    /// The layer's pixels, straight RGBA. Usually a decoded source frame.
    ///
    /// **Need not be the size of the canvas.** A source smaller than the canvas
    /// rests centred on it and covers only the pixels it has, leaving the rest
    /// of the canvas — and so the tracks below it — showing through; a larger
    /// one is clipped by the canvas edges. That is what a clip asking for its
    /// native size arrives as.
    pub source: &'a Frame,
    /// Where it goes, how big, how solid — already evaluated for this instant.
    /// The compositor animates nothing itself; it draws one moment.
    pub properties: Properties,
    /// Which edges of the frame the layer's own edges are measured from.
    ///
    /// Separate from [`Properties`] because it is not animated and must not
    /// become so: it says how the position is to be *read*, and animating that
    /// would move a layer by changing what its number means.
    pub anchor: Anchor,
    /// Which point of the layer's own raster its scale and rotation turn
    /// about.
    ///
    /// Separate from [`Properties`] for the reason [`Layer::anchor`] is: it
    /// says how a transform is to be *read*, and animating it would move a
    /// layer by changing what its numbers mean.
    pub origin: Origin,
}

impl<'a> Layer<'a> {
    /// A layer drawn exactly as it arrived.
    pub fn plain(source: &'a Frame) -> Self {
        Self {
            source,
            properties: Properties::default(),
            anchor: Anchor::default(),
            origin: Origin::default(),
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
    /// The canvas [`Frame`] disagrees with itself, which means something
    /// upstream resized one of the two without the other.
    #[error("a {resolution} canvas does not match its {bytes} bytes of buffer")]
    BadCanvas {
        /// The size the [`Frame`] claims.
        resolution: Resolution,
        /// The buffer it actually carries.
        bytes: usize,
    },
    /// The same disagreement in a [`Layer`]'s source — most often a decoded
    /// frame that did not arrive whole.
    #[error("a {resolution} layer does not match its {bytes} bytes of buffer")]
    BadLayer {
        /// The size the [`Frame`] claims.
        resolution: Resolution,
        /// The buffer it actually carries.
        bytes: usize,
    },
}
