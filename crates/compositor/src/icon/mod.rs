//! The symbols scorsese ships: [Lucide](https://lucide.dev), vendored whole.
//!
//! **The catalogue lives here and nowhere else**, which is the arrangement
//! `text::font::shipped` already has and for the same stated reason:
//! core defines property *types*, the compositor holds the property *values*.
//! A name like `clapperboard` is a value, so it belongs beside the code that
//! draws it — and adding one is a change to the vendored tree, never a change
//! to the format.
//!
//! A `project.json` reaches this through an `icon` asset, which carries a name
//! and nothing else about which symbol it is — so this module is the whole of
//! what "which symbols exist" means in this build: the set vendored, converted,
//! compiled in, drawable, and answerable when a document names one that is not
//! here.
//!
//! ## Why this set, and why bundled
//!
//! One library under one permissive licence. Lucide is ISC — commercial use,
//! no attribution required in the rendered artifact, which matters when the
//! artifact is an MP4 nobody can put a notice in. The licence text is vendored
//! beside the icons at `icons/LICENSE`, Feather's MIT section included, because
//! part of the set descends from it.
//!
//! Bundled because there is nothing else to be. A render that needed a network
//! to draw a triangle would break the offline guarantee the synthesis side is
//! built on, and upstream itself ships path data inline rather than fetching.
//!
//! ## What an icon is
//!
//! A 24×24 square, `fill: none`, `stroke: currentColor`, `stroke-width: 2`,
//! round caps and round joins. That is the whole visual vocabulary: **one
//! colour and one thickness**, which is one rounded stroke through `paint` and
//! nothing else. A [`Symbol`] chooses the colour, the thickness and how big the
//! square is in pixels; the drawing is the same either way.
//!
//! The handful of icons with a full stop in them — a dice pip, the dot of an
//! exclamation mark — carry it as a filled circle rather than a stroked one,
//! and those are an icon's filled contours. Same colour, no second choice.
//!
//! Tags and categories come along with the geometry. They are upstream's own,
//! they are what makes seventeen hundred icons findable at all, and dropping
//! them at conversion time is the decision that would have to be undone.

pub(crate) mod catalogue;
mod draw;
mod nearest;
mod search;

pub use search::Search;

use scorsese_core::{Anchor, Rgba};

use crate::area::Area;
use crate::frame::{Frame, Resolution};

/// The side of the square every icon is drawn in, in its own units.
pub const VIEWBOX: f32 = 24.0;

/// Lucide's own stroke width, in those same units.
const STROKE: f32 = 2.0;

/// One icon of the catalogue: what it is called, what it is about, and the
/// contours it draws.
///
/// Constructed only by reading the compiled-in blob, so every one of these is
/// `'static` and there is exactly one of each.
#[derive(Debug)]
pub struct Icon {
    name: &'static str,
    tags: Vec<&'static str>,
    categories: Vec<&'static str>,
    stroked: &'static [u8],
    filled: &'static [u8],
}

impl Icon {
    /// What a document will one day write to get it: upstream's file stem,
    /// lowercase and hyphenated.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The words upstream files it under — `clapperboard` carries *movie*,
    /// *film*, *cinema* and eight more. The set is unusable without them: no
    /// one is going to read seventeen hundred names.
    pub fn tags(&self) -> &[&'static str] {
        &self.tags
    }

    /// Upstream's broader grouping — *multimedia*, *arrows*, *weather*.
    pub fn categories(&self) -> &[&'static str] {
        &self.categories
    }

    /// How many path commands the icon draws, stroked and filled together.
    ///
    /// `None` for contours that do not decode, which is a blob that failed to
    /// parse rather than an icon that draws nothing — the vendoring tool
    /// refuses to write one of those.
    pub fn commands(&self) -> Option<usize> {
        Some(draw::count(self.stroked)? + draw::count(self.filled)?)
    }
}

/// A symbol to draw: which icon, how big, where, in what colour, how thick.
///
/// Everything is pixels, as it is for a [`Figure`](crate::shape::Figure): a
/// fraction of the frame is turned into one of these where the render
/// resolution is known.
#[derive(Debug, Clone, Copy)]
pub struct Symbol {
    /// Which one, from [`find`].
    pub icon: &'static Icon,
    /// The side of the square it is drawn in, in pixels.
    pub size: f32,
    /// Which edges of the frame that square is measured from.
    pub anchor: Anchor,
    /// The one colour it is drawn in, alpha included.
    pub color: Rgba,
    /// How thick the line is, in pixels — [`width_for`] is the thickness the
    /// icon was drawn at. A width that is zero, negative or not a number draws
    /// nothing, as it does for a border.
    pub width: f32,
}

/// The thickness an icon of this size was drawn at: Lucide's 2 units, scaled.
///
/// A starting point rather than a rule. A thicker line reads better on a busy
/// shot and a thinner one suits a large symbol, and both are the same drawing.
pub fn width_for(size: f32) -> f32 {
    size * STROKE / VIEWBOX
}

/// The icon of that name, if the set has one.
///
/// A binary search over the catalogue, which the blob stores in name order.
pub fn find(name: &str) -> Option<&'static Icon> {
    let icons = catalogue::all();
    let at = icons.binary_search_by(|icon| icon.name.cmp(name)).ok()?;
    icons.get(at)
}

/// Every icon, in name order.
///
/// Published because "which symbols are there?" is the question being asked
/// whenever somebody gets a name wrong, and — with [`Icon::tags`] — it is what
/// a search over the set is built from.
pub fn all() -> &'static [Icon] {
    catalogue::all()
}

/// The names closest to one the set does not have, best first, for the sentence
/// that refuses it.
///
/// Seventeen hundred names cannot be listed the way the shipped faces can, so
/// this is what a refusal says instead of *the ones scorsese ships are…*. It
/// catches a mistyped name and a half-remembered one, and it is empty when
/// there is honestly nothing near. It is **not** a search over what an icon is
/// about — see the module it lives in for why those are different questions.
pub fn nearest(name: &str) -> Vec<&'static str> {
    nearest::nearest(name)
}

/// The icons a word describes, best first — a substring of a name, a tag or a
/// category.
///
/// The way anything reaches this catalogue without reading it: seventeen
/// hundred names do not fit in a context window, and a caller that wants *the
/// film camera one* has no way to learn upstream calls it `clapperboard` short
/// of guessing names until one validates. Costs nothing and needs nothing — the
/// table is compiled in — so it is free to call on a hunch.
///
/// Not [`nearest`], which answers a different question: that one is for a name
/// that was **nearly right**, and it never looks at what an icon is about.
pub fn search(query: &str) -> Search {
    search::search(query)
}

/// The Lucide release the vendored tree came from.
///
/// Pinned, recorded in `icons/VERSION`, and moved deliberately: upstream
/// renames and retires icons, so a bump is a change with a note rather than a
/// re-sync.
pub fn version() -> &'static str {
    catalogue::version()
}

/// Draws a symbol onto a frame.
///
/// The frame is drawn onto as it arrives, cleared or not. An icon layer starts
/// transparent, so the tracks underneath show through everywhere the strokes
/// are not.
pub fn draw(frame: &mut Frame, symbol: &Symbol) {
    draw::draw(frame, symbol);
}

/// Where this symbol's square lands on the raster, in pixels.
///
/// `None` for a size that could not describe a square. This is the rectangle an
/// attached arrow meets — the icon's own box rather than the raster-sized layer
/// it is drawn into, because the box is what a reader sees and the layer's edges
/// are not on screen at all. The same answer [`crate::shape::area_of`] gives for
/// a box, for the same reason.
pub fn area_of(symbol: &Symbol, resolution: Resolution) -> Option<Area> {
    let size = symbol.size;
    if !size.is_finite() || size <= 0.0 {
        return None;
    }
    let (left, top) = draw::origin(size, symbol, resolution);
    Some(Area {
        left,
        top,
        width: size,
        height: size,
    })
}
