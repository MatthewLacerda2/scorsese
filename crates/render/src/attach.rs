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
//! **Two callers, one resolution.** The render redraws an attached arrow from
//! these ends every frame, and [`crate::follow`] reads them to find the line a
//! follower travels along — for the render and for [`crate::layout`] alike, so
//! the answer to *where is the packet* is the place it is drawn. Neither asks
//! in terms of the other's types: a layer is an index, its rectangle is
//! whatever the caller says sits at that index.
//!
//! **An arrow whose target is not on screen in this stretch is not drawn**, and
//! the render says so. The alternative — holding the endpoint where the box
//! would have been — draws a line into empty space, pointing at nothing, which
//! is a worse answer than an absent arrow and a note explaining it.

use scorsese_compositor::{Properties, Resolution};
use scorsese_core::{ClipId, Endpoint, Geometry, Shape, Side};

use crate::content::Rect;

/// One arrow that has to be redrawn as its ends move.
pub(crate) struct Following {
    /// The shape as authored, minus where its ends are.
    pub(crate) shape: Shape,
    /// Where the arrow starts.
    from: End,
    /// Where it ends.
    to: End,
}

impl Following {
    /// Where both ends are at the instant `properties` were resolved for, in
    /// the canvas's pixels.
    ///
    /// `properties` is the whole segment's, and `rect` says where the layer at
    /// an index sits within its own raster: an attached end reads where the
    /// layer it follows has got to.
    pub(crate) fn ends(
        &self,
        rect: impl Fn(usize) -> Rect,
        properties: &[Properties],
        canvas: Resolution,
    ) -> [(f32, f32); 2] {
        [&self.from, &self.to].map(|end| match end {
            End::Fixed(x, y) => (
                (x * f64::from(canvas.width())) as f32,
                (y * f64::from(canvas.height())) as f32,
            ),
            End::Follows { layer, side } => {
                rect(*layer).point_at(*side, &properties[*layer], canvas)
            }
        })
    }
}

impl Following {
    /// The layers its attached ends meet, by index.
    pub(crate) fn layers(&self) -> impl Iterator<Item = usize> + '_ {
        [&self.from, &self.to]
            .into_iter()
            .filter_map(|end| match end {
                End::Fixed(..) => None,
                End::Follows { layer, .. } => Some(*layer),
            })
    }
}

/// One end of an attached arrow, once the segment is known.
enum End {
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
/// `layer_of` says which layer of the segment a clip id names, if it is on
/// screen at all — and it looks **only among the layers drawn into the same
/// group as the arrow**. Validation refuses an arrow following a clip across a
/// group's edge, so that is not a filter that changes an answer — it is what
/// keeps the answer right when one group is on screen twice at once, and its
/// members' ids with it: each copy of the arrow follows the box in its own copy
/// of the group.
pub(crate) fn following(
    shape: &Shape,
    layer_of: impl Fn(&ClipId) -> Option<usize>,
) -> Option<Following> {
    let Geometry::Arrow { from, to, .. } = &shape.geometry else {
        return None;
    };
    let end = |endpoint: &Endpoint| match endpoint {
        Endpoint::At(point) => Some(End::Fixed(point.x, point.y)),
        Endpoint::Attached { attach } => layer_of(&attach.clip).map(|layer| End::Follows {
            layer,
            side: attach.side,
        }),
    };
    Some(Following {
        shape: shape.clone(),
        from: end(from)?,
        to: end(to)?,
    })
}

/// True when any end of this arrow follows a clip rather than naming a place.
pub(crate) fn is_attached(shape: &Shape) -> bool {
    let Geometry::Arrow { from, to, .. } = &shape.geometry else {
        return false;
    };
    from.attach().is_some() || to.attach().is_some()
}
