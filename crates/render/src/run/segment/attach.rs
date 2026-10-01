//! Arrows that follow the clips they point at.
//!
//! **This is the first place in the renderer where one layer's geometry depends
//! on another's**, and the whole design here is about paying for that as little
//! as possible. An attachment is resolved to a *layer of this segment* — an
//! index into the same slots and the same per-frame properties every other
//! layer already has — so no ordering is introduced, nothing is computed twice,
//! and there is no graph to walk. Core refuses an arrow attached to an arrow,
//! which is what makes that safe.
//!
//! **An arrow whose target is not on screen in this stretch is not drawn**, and
//! the render says so. The alternative — holding the endpoint where the box
//! would have been — draws a line into empty space, pointing at nothing, which
//! is a worse answer than an absent arrow and a note explaining it.

use scorsese_compositor::{Properties, Resolution};
use scorsese_core::{Attach, Endpoint, Geometry, Shape, Side};

use super::layers::{Entry, Slot};

/// Where a layer sits within its own raster, and how to ask where that lands
/// on the canvas. Shared with the query that reports the same rectangle
/// without drawing anything — [`crate::content`] says why there is only one of
/// them.
pub(super) use crate::content::Rect;

/// One arrow that has to be redrawn as its ends move.
pub(super) struct Following {
    /// The shape as authored, minus where its ends are.
    pub(super) shape: Shape,
    /// Where the arrow starts.
    pub(super) from: End,
    /// Where it ends.
    pub(super) to: End,
}

impl Following {
    /// Where both ends are at the instant `properties` were resolved for, in
    /// the canvas's pixels.
    ///
    /// `properties` is the whole segment's, in the same order as `slots`: an
    /// attached end reads where the layer it follows has got to.
    pub(super) fn ends(
        &self,
        slots: &[Slot],
        properties: &[Properties],
        canvas: Resolution,
    ) -> [(f32, f32); 2] {
        [&self.from, &self.to].map(|end| match end {
            End::Fixed(x, y) => (
                (x * f64::from(canvas.width())) as f32,
                (y * f64::from(canvas.height())) as f32,
            ),
            End::Follows { layer, side } => {
                slots[*layer]
                    .rect
                    .point_at(*side, &properties[*layer], canvas)
            }
        })
    }
}

/// One end of an attached arrow, once the segment is known.
pub(super) enum End {
    /// A place on the frame, as fractions — the same thing an unattached arrow
    /// has, carried through unchanged.
    Fixed(f64, f64),
    /// A layer of this segment, and which side of it to meet.
    Follows {
        /// Which layer, by its index among this segment's slots.
        layer: usize,
        /// Which side of that layer's own rectangle.
        side: Side,
    },
}

/// Turns an arrow's authored endpoints into ends this segment can resolve.
///
/// `None` when an end names a clip that is not on screen here — the whole arrow
/// is dropped rather than half of it drawn, since half an arrow is a line
/// pointing away from nothing.
///
/// `within` is the group the arrow is drawn into, and an end is looked for
/// **only among the layers drawn into the same one**. Validation refuses an
/// arrow following a clip across a group's edge, so this is not a filter that
/// changes an answer — it is what keeps the answer right when one group is on
/// screen twice at once, and its members' ids with it: each copy of the arrow
/// follows the box in its own copy of the group.
pub(super) fn following(
    shape: &Shape,
    layers: &[Entry<'_, '_>],
    within: Option<usize>,
) -> Option<Following> {
    let Geometry::Arrow { from, to, .. } = &shape.geometry else {
        return None;
    };
    Some(Following {
        shape: shape.clone(),
        from: end(from, layers, within)?,
        to: end(to, layers, within)?,
    })
}

fn end(endpoint: &Endpoint, layers: &[Entry<'_, '_>], within: Option<usize>) -> Option<End> {
    match endpoint {
        Endpoint::At(point) => Some(End::Fixed(point.x, point.y)),
        Endpoint::Attached { attach } => {
            layer_of(attach, layers, within).map(|layer| End::Follows {
                layer,
                side: attach.side,
            })
        }
    }
}

/// Which layer of this segment a clip id names, among those drawn into the
/// same group, if it is on screen at all.
fn layer_of(attach: &Attach, layers: &[Entry<'_, '_>], within: Option<usize>) -> Option<usize> {
    layers
        .iter()
        .position(|entry| entry.within == within && entry.shot.clip.id == attach.clip)
}

/// True when any end of this arrow follows a clip rather than naming a place.
pub(super) fn is_attached(shape: &Shape) -> bool {
    let Geometry::Arrow { from, to, .. } = &shape.geometry else {
        return false;
    };
    from.attach().is_some() || to.attach().is_some()
}
