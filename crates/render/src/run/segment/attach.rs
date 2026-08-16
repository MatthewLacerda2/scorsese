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

use scorsese_compositor::{Area, Properties, Resolution};
use scorsese_core::{Anchor, Attach, Endpoint, Geometry, Shape, Side};

use crate::plan::Shot;

/// One arrow that has to be redrawn as its ends move.
pub(super) struct Following {
    /// The shape as authored, minus where its ends are.
    pub(super) shape: Shape,
    /// Where the arrow starts.
    pub(super) from: End,
    /// Where it ends.
    pub(super) to: End,
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

/// Where a layer sits, everything an attachment needs to ask about it.
///
/// Two rectangles are in play and confusing them is the bug this type exists to
/// prevent. `source` is the raster the layer's pixels live on; `area` is the
/// part of it the layer actually *shows* — a title's wrapped block, a box's own
/// box — which for a decoded picture is all of it and for a shape is very much
/// not.
#[derive(Debug, Clone, Copy)]
pub(super) struct Rect {
    /// The layer's own raster.
    pub(super) source: Resolution,
    /// What it shows within that raster.
    pub(super) area: Area,
    /// Which edges of the frame it rests against.
    pub(super) anchor: Anchor,
}

impl Rect {
    /// Where `side` of this layer is on the canvas, for one instant.
    ///
    /// The layer's own transform is applied by the compositor's matrix rather
    /// than a copy of it: a second implementation of *where does a layer land*
    /// would drift from the first, and the drift would look like an arrow
    /// missing its box by a few pixels.
    pub(super) fn point_at(
        self,
        side: Side,
        properties: &Properties,
        canvas: Resolution,
    ) -> (f32, f32) {
        let (across, down) = side.on_unit_rect();
        scorsese_compositor::on_canvas(
            properties,
            self.anchor,
            self.source,
            canvas,
            self.area.at(across, down),
        )
    }
}

/// Turns an arrow's authored endpoints into ends this segment can resolve.
///
/// `None` when an end names a clip that is not on screen here — the whole arrow
/// is dropped rather than half of it drawn, since half an arrow is a line
/// pointing away from nothing.
pub(super) fn following(shape: &Shape, layers: &[Shot<'_>]) -> Option<Following> {
    let Geometry::Arrow { from, to, .. } = &shape.geometry else {
        return None;
    };
    Some(Following {
        shape: shape.clone(),
        from: end(from, layers)?,
        to: end(to, layers)?,
    })
}

fn end(endpoint: &Endpoint, layers: &[Shot<'_>]) -> Option<End> {
    match endpoint {
        Endpoint::At(point) => Some(End::Fixed(point.x, point.y)),
        Endpoint::Attached { attach } => layer_of(attach, layers).map(|layer| End::Follows {
            layer,
            side: attach.side,
        }),
    }
}

/// Which layer of this segment a clip id names, if it is on screen at all.
fn layer_of(attach: &Attach, layers: &[Shot<'_>]) -> Option<usize> {
    layers.iter().position(|shot| shot.clip.id == attach.clip)
}

/// True when any end of this arrow follows a clip rather than naming a place.
pub(super) fn is_attached(shape: &Shape) -> bool {
    let Geometry::Arrow { from, to, .. } = &shape.geometry else {
        return false;
    };
    from.attach().is_some() || to.attach().is_some()
}
