//! A rectangle of a frame, in pixels.
//!
//! One type rather than one per caller, because the question every caller is
//! asking is the same one: *where on this raster is that thing?* A title's
//! wrapped block and a shape's box are both answers to it, and an arrow
//! attaching to either has to read them the same way.

/// A rectangle on a raster, in pixels from its top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    /// Pixels from the frame's left edge to this rectangle's.
    pub left: f32,
    /// Pixels from the frame's top edge to this rectangle's.
    pub top: f32,
    /// How wide it is.
    pub width: f32,
    /// How tall it is.
    pub height: f32,
}

impl Area {
    /// The whole of a raster — what a layer occupies when it has no smaller
    /// rectangle of its own, which is every decoded picture and every colour.
    pub fn whole(resolution: crate::frame::Resolution) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            width: resolution.width() as f32,
            height: resolution.height() as f32,
        }
    }

    /// The point at `(across, down)` of this rectangle, both fractions of it:
    /// `(0, 0)` its top-left corner and `(1, 1)` its bottom-right.
    pub fn at(self, across: f64, down: f64) -> (f32, f32) {
        (
            self.left + self.width * across as f32,
            self.top + self.height * down as f32,
        )
    }
}

/// Where a point of a layer's **own raster** lands on the canvas.
///
/// The layer's transform is the compositor's own — the same matrix that draws
/// it — so anything asking where a layer's edge ended up gets the answer the
/// picture actually has. A second implementation would drift from the first,
/// and the drift would show as an arrow missing the box it is attached to.
///
/// `source` is the layer's raster and `canvas` the frame it is drawn onto;
/// `point` is in `source`'s pixels.
pub fn on_canvas(
    properties: &crate::properties::Properties,
    anchor: scorsese_core::Anchor,
    origin: scorsese_core::Origin,
    source: crate::frame::Resolution,
    canvas: crate::frame::Resolution,
    point: (f32, f32),
) -> (f32, f32) {
    let transform = crate::cpu::transform_of(properties, anchor, origin, source, canvas);
    let mut mapped = [tiny_skia::Point::from_xy(point.0, point.1)];
    transform.map_points(&mut mapped);
    (mapped[0].x, mapped[0].y)
}
