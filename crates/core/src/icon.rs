//! Symbols the render draws, named rather than imported.
//!
//! The fourth kind with no file behind it, after `text`, `color` and `shape`,
//! and it is here for the reason they are: a play triangle or a clapperboard
//! authored as a PNG somewhere else is a megabyte in `assets/`, soft at the
//! next resolution, and unchangeable in colour without going back to the other
//! program. A **name** is a few bytes, sharp at 4K, and recoloured by editing
//! one string — and it survives `scp -r` between machines, because the symbols
//! ship with the binary exactly as the faces a `style` names do.
//!
//! **The generality rule holds, and this is where it is easiest to break.**
//! Core says an icon *has* a name, a size, a colour and a thickness. It never
//! says which names exist: that is a set of property *values*, and it lives in
//! the compositor beside the catalogue that draws them — the same split
//! `text::font::shipped` documents on the other side. So an unknown name is not
//! a validation error here; it is refused where the catalogue is, by
//! `scorsese check` and by the render.
//!
//! **Not a fourth [`Geometry`](crate::Geometry), deliberately.** A
//! [`Shape`](crate::Shape)'s `stroke_width` is a fraction of the *raster's*
//! height and so does not scale with the shape — right for a callout box, which
//! wants the same visible border whatever size it is, and wrong for a symbol,
//! which shrunk into a corner would fill in solid and read as a blob. And
//! `fill` means nothing on an open Lucide stroke, so it has to be refused
//! rather than quietly ignored. Same field names, opposite meanings.
//!
//! Nothing here animates. An icon that grows or fades does it through the
//! properties every layer already has — `transform.scale` and `opacity` — as a
//! shape does.

use serde::{Deserialize, Serialize};

use crate::color::Rgba;

/// How thick an icon's line is when the document does not say: Lucide's own
/// `2` units in a `24`-unit box, which is what every symbol was drawn at.
pub const DEFAULT_ICON_STROKE_WIDTH: f64 = 2.0 / 24.0;

/// What an `icon` asset is: which symbol, how big, in what colour, how thick.
///
/// Four numbers and a name, and no file anywhere — which is the whole of the
/// bargain. There is nothing to import, nothing to hash, nothing to probe, and
/// no resolution the document can be wrong at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Icon {
    /// Which symbol, by the catalogue's own name for it — lowercase and
    /// hyphenated, `clapperboard` or `circle-play`.
    ///
    /// A name and not a path, which is what makes a project portable: the
    /// symbols ship with the binary, so nothing has to travel beside the
    /// document. Which names there are is not this crate's to say — see the
    /// module doc.
    pub name: String,
    /// How big the square it is drawn in is, as a fraction of the raster's
    /// **height**.
    ///
    /// **One number against one axis, and the icon stays square** — which is
    /// where this differs from the neighbouring kind, and the difference is
    /// worth reading twice. A [`Shape`](crate::Shape) takes a `width` against
    /// the frame's width and a `height` against its height, because a
    /// rectangle has two independent sides. An icon does not: it is drawn in a
    /// 24-unit square, so it needs one measurement, and the axis chosen for it
    /// is the one a text `size` already uses. The same number of pixels comes
    /// out both ways, on every aspect ratio.
    pub size: f64,
    /// The one colour it is drawn in, alpha included.
    ///
    /// One, because a symbol has one: the whole vocabulary is a single stroke
    /// in a single colour, and a two-colour icon is a different feature. No
    /// default, for the reason a `color` asset has none — a symbol nobody chose
    /// the colour of would come out as *some* colour, and the shot would simply
    /// be wrong with nothing to say so.
    pub color: Rgba,
    /// How thick that line is, as a fraction of the **icon's own box** —
    /// [`DEFAULT_ICON_STROKE_WIDTH`] when the document does not say.
    ///
    /// **Relative to the icon, so it scales with it.** That is the opposite
    /// choice from [`Shape::stroke_width`](crate::Shape::stroke_width), which
    /// is a fraction of the raster and deliberately keeps a constant visible
    /// weight whatever size the box is. A border wants that; a symbol does not.
    /// Halve an icon with a raster-relative stroke and the line stays as thick
    /// while the drawing halves around it, until the counters close up and the
    /// symbol reads as a blob. Written as a fraction of the box, a half-size
    /// icon is simply the same picture, half the size.
    #[serde(default = "default_stroke_width")]
    pub stroke_width: f64,
}

fn default_stroke_width() -> f64 {
    DEFAULT_ICON_STROKE_WIDTH
}

impl Icon {
    /// A symbol at Lucide's own thickness — the common case, and the one a
    /// document gets by leaving `stroke_width` out.
    pub fn new(name: impl Into<String>, size: f64, color: Rgba) -> Self {
        Self {
            name: name.into(),
            size,
            color,
            stroke_width: DEFAULT_ICON_STROKE_WIDTH,
        }
    }

    /// The same symbol drawn at another weight, as a fraction of its own box.
    #[must_use]
    pub fn weighing(self, stroke_width: f64) -> Self {
        Self {
            stroke_width,
            ..self
        }
    }

    /// Whether anything would be drawn at all.
    ///
    /// The one thing an icon cannot be allowed to be, for the reason a shape
    /// cannot be invisible: a layer that renders nothing looks exactly like a
    /// layer that failed to render. A size or a thickness that is zero,
    /// negative, or not a number at all is each on its own enough to produce
    /// one.
    pub fn draws(&self) -> bool {
        positive(self.size) && positive(self.stroke_width)
    }
}

/// A measurement that could describe something with ink in it — which rules
/// out zero, negatives, and the two ends of the float range an arithmetic
/// mistake arrives at without anyone having written them down.
fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}
