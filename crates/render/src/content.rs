//! Which rectangle of its own raster a layer's **content** actually covers,
//! and where that rectangle lands on the canvas.
//!
//! Two callers ask this and they must never disagree. The render asks so that
//! an arrow can attach to a box and follow it; [`crate::layout`] asks on behalf
//! of somebody authoring a layout, who wants the answer without rendering a
//! frame and looking at it. A second implementation of *where does this layer
//! land* would drift from the first, and the drift would be an editor that
//! reports one place and draws another.
//!
//! So the decision — a title's wrapped block, a shape's own box, a card, a
//! fitted picture — is made once, in [`within`], and the mapping onto the
//! canvas goes through the compositor's own matrix
//! ([`scorsese_compositor::on_canvas`]) rather than a copy of it.

use std::path::Path;

use scorsese_compositor::{Area, Properties, Resolution};
use scorsese_core::{Anchor, AssetKind, Clip, Frames, Origin, Side};

use crate::error::RenderError;
use crate::plan::Shot;
use crate::slug::{self, Standing};
use crate::text::Painter;

/// Where a layer sits, everything an attachment or a query needs to ask about
/// it.
///
/// Two rectangles are in play and confusing them is the bug this type exists to
/// prevent. `source` is the raster the layer's pixels live on; `area` is the
/// part of it the layer actually *shows* — a title's wrapped block, a box's own
/// box — which for a decoded picture is all of it and for a shape is very much
/// not.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    /// The layer's own raster.
    pub(crate) source: Resolution,
    /// What it shows within that raster.
    pub(crate) area: Area,
    /// Which edges of the frame it rests against.
    pub(crate) anchor: Anchor,
    /// Which point of the layer its scale and rotation pivot on — part of the
    /// same matrix, so an attachment that ignored it would meet a box that is
    /// no longer where it was drawn.
    pub(crate) origin: Origin,
}

impl Rect {
    /// Where `side` of this layer is on the canvas, for one instant.
    ///
    /// The layer's own transform is applied by the compositor's matrix rather
    /// than a copy of it: a second implementation of *where does a layer land*
    /// would drift from the first, and the drift would look like an arrow
    /// missing its box by a few pixels.
    pub(crate) fn point_at(
        self,
        side: Side,
        properties: &Properties,
        canvas: Resolution,
    ) -> (f32, f32) {
        let (across, down) = side.on_unit_rect();
        self.on_canvas(properties, canvas, (across, down))
    }

    /// The upright rectangle of the canvas this layer's content covers at one
    /// instant, in the canvas's own pixels.
    ///
    /// **Upright, and so a bounding box.** A turned layer's content is a turned
    /// rectangle, which is not something a caller comparing two layouts can
    /// use; the smallest upright rectangle containing it is, and it is the
    /// honest reading of "how much of the frame does this cover". All four
    /// corners go through the same matrix the drawing does, so a layer that is
    /// merely moved or scaled — which is nearly all of them — comes back exact.
    pub(crate) fn bounds_on(self, properties: &Properties, canvas: Resolution) -> Area {
        let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
            .map(|corner| self.on_canvas(properties, canvas, corner));
        let left = corners
            .iter()
            .map(|corner| corner.0)
            .fold(f32::MAX, f32::min);
        let right = corners
            .iter()
            .map(|corner| corner.0)
            .fold(f32::MIN, f32::max);
        let top = corners
            .iter()
            .map(|corner| corner.1)
            .fold(f32::MAX, f32::min);
        let bottom = corners
            .iter()
            .map(|corner| corner.1)
            .fold(f32::MIN, f32::max);
        Area {
            left,
            top,
            width: right - left,
            height: bottom - top,
        }
    }

    /// One point of this rectangle, given as fractions of it, on the canvas.
    fn on_canvas(
        self,
        properties: &Properties,
        canvas: Resolution,
        (across, down): (f64, f64),
    ) -> (f32, f32) {
        scorsese_compositor::on_canvas(
            properties,
            self.anchor,
            self.origin,
            self.source,
            canvas,
            self.area.at(across, down),
        )
    }
}

/// What a layer shows, as far as *where it is* is concerned.
///
/// Three answers rather than one rectangle, because two of the cases genuinely
/// are not a rectangle of the render's raster: decoded media covers whatever
/// raster its frames arrive at, which only the caller reading the pipe knows,
/// and an arrow has no box at all.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Content {
    /// A layer drawn at the render's own raster, and the rectangle its content
    /// covers within it.
    Drawn(Area),
    /// Decoded media, which covers the whole of whatever raster its frames
    /// arrive at — for a letterboxed `fit` that is the picture rather than the
    /// canvas.
    Media,
    /// A layer with no rectangle of its own: an arrow, which is a line between
    /// two points rather than a box.
    Unbounded,
}

impl Content {
    /// The rectangle within a raster-sized layer, taking the whole raster for
    /// the two cases that have no box of their own.
    ///
    /// What the render does with them, and it is the right answer there: a
    /// raster-sized layer is composited as a raster-sized layer whatever is
    /// drawn on it.
    pub(crate) fn or_whole(self, raster: Resolution) -> Area {
        match self {
            Self::Drawn(area) => area,
            Self::Media | Self::Unbounded => Area::whole(raster),
        }
    }
}

/// What this shot's content covers, within the raster its layer is drawn at.
///
/// `raster` is the render's own, which is what every drawn layer is the size
/// of. Nothing is decoded and no process is spawned; a text asset's block is
/// measured by setting it, through the same painter that would draw it.
pub(crate) fn within(
    shot: &Shot<'_>,
    painter: &mut Painter,
    raster: Resolution,
    project_root: &Path,
) -> Result<Content, RenderError> {
    if shot.asset.kind == AssetKind::Text {
        // The wrapped block, not the raster it is set on: the words are what a
        // reader sees and what an arrow has to meet, and the layer's own edges
        // are the frame's.
        let block = painter.block(shot.asset, shot.clip.anchor, project_root, raster)?;
        return Ok(Content::Drawn(block));
    }
    if shot.asset.kind == AssetKind::Color {
        // A colour *is* the whole raster, so its rectangle is too.
        return Ok(Content::Drawn(Area::whole(raster)));
    }
    if let Some(icon) = &shot.asset.icon {
        // The symbol's own square, not the raster it is drawn into — what a
        // reader sees, and the only rectangle an arrow could sensibly meet. An
        // icon is always bounded: unlike an arrow, there is no open-ended
        // geometry here, so a missing square means the raster is degenerate
        // rather than that the drawing has no box.
        return Ok(crate::symbol::area(icon, raster, shot.clip.anchor)
            .map_or(Content::Unbounded, Content::Drawn));
    }
    if let Some(shape) = &shot.asset.shape {
        // A box's own box, not the raster it is drawn into. `None` is an
        // arrow, which is the one drawn thing with no box to report.
        return Ok(crate::shape::area(shape, raster, shot.clip.anchor)
            .map_or(Content::Unbounded, Content::Drawn));
    }
    match slug::standing(shot, project_root)? {
        // A card is a panel of the raster: all of it for a picture, a band
        // across the foot for a narration prompt that has to share the frame
        // with the shot it narrates.
        Standing::Card(_) => Ok(Content::Drawn(slug::area(shot.asset, raster))),
        Standing::Media(_) => Ok(Content::Media),
    }
}

/// How far into its own clip a timeline frame is.
pub(crate) fn elapsed(at: Frames, clip: &Clip) -> Frames {
    Frames(at.get().saturating_sub(clip.start.get()))
}
