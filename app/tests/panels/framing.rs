//! How much of the window each snapshot is a picture of (#755).
//!
//! Every reference used to be the **whole window**, so one button added to the
//! top bar re-blessed twelve of sixteen pictures that were about something else
//! (#726), and two branches that each moved a shared strip conflicted in PNGs
//! nobody can merge (#730). Now each picture is cropped to the part it is named
//! for, and a change to the bar re-blesses only pictures *of* the bar.
//!
//! Cropping everything would lose what a whole window catches and a part never
//! can: a **layout** regression — a panel running under another, the side
//! column overrunning the window (#681). So two references stay whole on
//! purpose, `a_whole_edit` and `a_whole_edit_in_light`, and they are the only
//! ones a change to the chrome is expected to move.

use egui::{Context, Id, PanelState, Rect};
use image::RgbaImage;

/// A part of the window a snapshot is cropped to.
///
/// Named by what a person would call it rather than by egui's panel ids, which
/// are the app's business — the ids are looked up in one place, [`Part::rect`].
#[derive(Clone, Copy, Debug)]
pub(crate) enum Part {
    /// All of it. The layout check, and only that: see the module doc.
    Window,
    /// The strip along the top — the wordmark, the project's name, and the
    /// buttons that are not edits.
    Bar,
    /// The column down the right: the inspector over the project files.
    Side,
    /// The tracks along the bottom, from the ruler down.
    Timeline,
    /// Whatever is left in the middle: the preview, or what is said instead.
    Centre,
    /// A floating window, by its title.
    Dialog(&'static str),
}

impl Part {
    /// Where this part was drawn on the last frame, in points.
    ///
    /// Read from what egui remembered of that frame rather than computed from
    /// the app's constants, so a panel that is resized, or sizes itself to its
    /// content, is cropped where it actually is.
    fn rect(self, ctx: &Context) -> Rect {
        let panel = |id: &str| {
            PanelState::load(ctx, Id::new(id))
                .unwrap_or_else(|| panic!("the `{id}` panel was drawn"))
                .outer_rect
        };
        match self {
            Self::Window => ctx.content_rect(),
            Self::Bar => panel("menu"),
            Self::Side => panel("side"),
            Self::Timeline => panel("timeline"),
            // The central panel keeps no state of its own; it is exactly the
            // room the edge panels leave. Its left edge is the timeline's
            // rather than the window's, because the harness draws inside a
            // margin and the panels start where that margin ends.
            Self::Centre => Rect::from_min_max(
                egui::pos2(panel("timeline").left(), panel("menu").bottom()),
                egui::pos2(panel("side").left(), panel("timeline").top()),
            ),
            // A window's area is keyed by its title as egui reads it, which is
            // an optional text — hence the `Some`.
            Self::Dialog(title) => ctx
                .memory(|memory| memory.area_rect(Id::new(Some(title))))
                .unwrap_or_else(|| panic!("the `{title}` window is open")),
        }
    }

    /// `image`, which is the whole window, cut down to this part.
    pub(crate) fn crop(self, ctx: &Context, image: &RgbaImage) -> RgbaImage {
        let scale = ctx.pixels_per_point();
        let rect = self.rect(ctx);
        // Rounded outwards to whole pixels and held inside the image, so a
        // panel edge on a half point keeps its hairline rather than losing it.
        let edge = |pixel: f32, most: u32| (pixel.max(0.0) as u32).min(most);
        let (width, height) = image.dimensions();
        let left = edge((rect.left() * scale).floor(), width);
        let top = edge((rect.top() * scale).floor(), height);
        let right = edge((rect.right() * scale).ceil(), width);
        let bottom = edge((rect.bottom() * scale).ceil(), height);
        assert!(
            left < right && top < bottom,
            "{self:?} is somewhere on screen, not {rect:?}"
        );
        image::imageops::crop_imm(image, left, top, right - left, bottom - top).to_image()
    }
}
