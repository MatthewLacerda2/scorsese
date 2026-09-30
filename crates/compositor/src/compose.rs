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
    /// Which point of the layer's own raster its scale, rotation and flip
    /// turn about.
    ///
    /// Separate from [`Properties`] for the reason [`Layer::anchor`] is: it
    /// says how a transform is to be *read*, and animating it would move a
    /// layer by changing what its numbers mean.
    pub origin: Origin,
    /// The layer this one is shown through, if any — see [`Matte`].
    ///
    /// Boxed because a matte is a layer too, and a layer cannot hold itself
    /// by value. A matte's own `matte` is ignored: a chain of them is refused
    /// long before anything is drawn, and drawing one would be a compositing
    /// graph this crate does not have.
    pub matte: Option<Box<Matte<'a>>>,
}

/// A layer drawn only where another layer's picture is — a track matte.
///
/// **The matte is drawn, not read.** Its layer goes through every stage any
/// layer does — its own transform, blur, opacity, even its light — onto a
/// transparent raster the size of the canvas, and the **alpha** of that
/// picture is the mask. So a matte moves, scales, softens and fades the way
/// its clip does, and a wipe is nothing more than a rectangle whose scale is
/// keyframed.
///
/// **It is the last thing that happens to the masked layer**, applied as the
/// layer lands on the canvas: after its shadow and glow, and after its
/// transform. That order is forced rather than chosen — the two layers have
/// transforms of their own and only meet on the canvas — and it is also the
/// one that reads right: a matte reveals the layer *with* its light, where
/// cutting the layer before its shadow was grown would leave the shadow of
/// the whole layer hanging outside the reveal.
#[derive(Debug, Clone, PartialEq)]
pub struct Matte<'a> {
    /// What the mask is drawn from.
    pub layer: Layer<'a>,
    /// Show the masked layer where the matte is **not**, rather than where it
    /// is.
    pub invert: bool,
}

impl<'a> Layer<'a> {
    /// A layer drawn exactly as it arrived.
    pub fn plain(source: &'a Frame) -> Self {
        Self {
            source,
            properties: Properties::default(),
            anchor: Anchor::default(),
            origin: Origin::default(),
            matte: None,
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

    /// Draws `layers` onto `canvas` exactly as [`Compositor::composite`]
    /// does, **except that the canvas starts transparent rather than black**,
    /// and what is left on it is straight RGBA — so the result can be the
    /// `source` of another [`Layer`].
    ///
    /// **This is the seam for anything that has to treat several layers, or
    /// one layer on its own, as a single picture before it meets the frame.**
    /// A group is the first: its members are drawn here, and the result is
    /// composited once with the group clip's own transform and opacity — which
    /// is why a group at half opacity does not show its overlapping members
    /// through each other, the tell-tale that it is one layer. An effect that
    /// needs a layer's own pixels in isolation — a glow grown off its alpha, a
    /// matte cut from another layer — renders that layer here first, for the
    /// same reason: so it acts on the finished picture of the thing and never
    /// on the frame beneath it.
    ///
    /// Where nothing covers the canvas it stays fully transparent, and the
    /// tracks below the layer made from it show through.
    fn offscreen(&mut self, canvas: &mut Frame, layers: &[Layer<'_>])
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
