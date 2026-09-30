//! The layers drawn again for every frame of a segment, and why each one is.
//!
//! Almost everything drawn — a title, a colour, a box — is the same pixels for
//! the whole segment and is drawn once. Two things are not, and they are not
//! for different reasons: an arrow following a clip changes because *another*
//! layer moved, and a shape whose line is keyframed changes because *its own*
//! trim or dash offset did. Both end up in the same place — a buffer of the
//! job's own, filled once the frame's properties are resolved — so they share
//! one pass and one variant of [`super::layers::Pixels`].

use scorsese_compositor::{Frame, Properties, Resolution};
use scorsese_core::Shape;

use super::attach::Following;
use super::layers::Slot;

/// What a per-frame layer is redrawn from.
pub(super) enum Redraw {
    /// An arrow with an end that follows a clip, and where each end comes from.
    /// Its own line may be keyframed as well; that costs nothing extra here.
    Following(Following),
    /// A shape whose line is keyframed — `shape.trim_*` or `shape.dash_offset`
    /// — and nothing else about it moves.
    Traced(Shape),
}

impl Redraw {
    /// Draws layer `own` of `slots` into `frame`, at the instant `properties`
    /// were resolved for.
    ///
    /// `properties` is the whole segment's, in the same order as `slots`: an
    /// attached end reads where the layer it follows has got to, and the layer
    /// reads its own trace from its own entry.
    pub(super) fn draw(
        &self,
        frame: &mut Frame,
        slots: &[Slot],
        properties: &[Properties],
        own: usize,
        canvas: Resolution,
    ) {
        let trace = properties[own].trace;
        match self {
            Self::Following(following) => {
                let ends = following.ends(slots, properties, canvas);
                crate::shape::paint_arrow(frame, &following.shape, ends, trace);
            }
            Self::Traced(shape) => crate::shape::paint(frame, shape, slots[own].anchor, trace),
        }
    }
}
