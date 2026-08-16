//! Shapes the render draws, rather than pictures of shapes somebody imported.
//!
//! The third kind with no file behind it, after `text` and `color`, and it is
//! here for the same reason they are: a box with a black border is a thing you
//! would otherwise author as a PNG in another program, import a megabyte of,
//! and then find wrong the moment the render resolution changed. Everything
//! here is a fraction of the raster, so one document reads the same at 640×360
//! and at 4K, and the drawing happens at whatever size the render is — which is
//! what makes the edge of a circle stay clean instead of stepping.
//!
//! **The generality rule holds as plainly as it does for text.** Core says a
//! shape *has* an outline, an interior colour and a border colour; it never
//! says which. There is no "callout box" and no "make it red".
//!
//! Nothing here animates. A shape that fades or slides does it through the
//! properties every layer already has — `opacity` and `transform.*` — rather
//! than through a second, shape-shaped way of saying the same thing.

use serde::{Deserialize, Serialize};

use crate::color::Rgba;

/// How thick a border is when the document gives it a colour and no width:
/// about four pixels at 1080p, which is a line you can see without it becoming
/// the subject.
pub const DEFAULT_STROKE_WIDTH: f64 = 0.004;

/// The most a corner can be rounded — half the shape's shorter side, where the
/// two corners of that side meet and the end has become a semicircle. Past it
/// there is no straight edge left to round.
pub const MAX_RADIUS: f64 = 0.5;

/// What a `shape` asset is: an outline, and how it is coloured.
///
/// The interior and the border are separate colours and either may be absent,
/// because that is most of the use. A border over an absent fill is a callout
/// that does not hide the footage inside it; a fill with no border is a plain
/// block. Both absent would draw nothing at all, which
/// [`crate::Project::validate`] refuses rather than rendering an empty layer
/// nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    /// The outline itself — the only part a rectangle and an ellipse disagree
    /// about.
    pub geometry: Geometry,
    /// What the inside is painted. Absent leaves it see-through, so whatever
    /// the shape is drawn over shows through the middle of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Rgba>,
    /// What the border is drawn in. Absent means no border at all, which is
    /// not the same as a border of width zero — the difference matters only in
    /// that one is said and the other is drawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Rgba>,
    /// How thick that border is, as a fraction of the raster's **height** —
    /// the unit a text `size` already uses, and for the same reason: a
    /// thickness has no axis of its own, so it needs one chosen for it, and
    /// choosing the same one twice is one fewer thing to remember.
    ///
    /// The border straddles the outline, half inside and half out, which is
    /// what every drawing program does and what keeps a shape's stated size the
    /// size of the shape rather than of its ink.
    #[serde(default = "default_stroke_width")]
    pub stroke_width: f64,
}

fn default_stroke_width() -> f64 {
    DEFAULT_STROKE_WIDTH
}

impl Shape {
    /// A shape with an interior and no border.
    pub fn filled(geometry: Geometry, fill: Rgba) -> Self {
        Self {
            geometry,
            fill: Some(fill),
            stroke: None,
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }

    /// A shape with a border and a see-through inside — the callout case.
    pub fn outlined(geometry: Geometry, stroke: Rgba) -> Self {
        Self {
            geometry,
            fill: None,
            stroke: Some(stroke),
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }

    /// The same shape with a border added, so the two-colour case reads as what
    /// it is rather than as a struct literal.
    #[must_use]
    pub fn bordered(self, stroke: Rgba, width: f64) -> Self {
        Self {
            stroke: Some(stroke),
            stroke_width: width,
            ..self
        }
    }

    /// Whether anything would be drawn at all. The one thing a shape cannot be
    /// allowed to be, since a layer that renders nothing looks exactly like a
    /// layer that failed to render.
    pub fn draws(&self) -> bool {
        self.fill.is_some() || self.stroke.is_some()
    }
}

/// The outline a shape has.
///
/// Two of them, and the list is meant to stay short. A rectangle and an ellipse
/// are what a diagram is made of; polygons, stars and arbitrary paths are a
/// drawing program growing inside a video editor. Lines and arrows are the one
/// planned addition, and they are a different shape of thing entirely — two
/// endpoints rather than a size — which is why they are not here yet.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Geometry {
    /// Four corners, as square or as rounded as the document says.
    Rectangle {
        /// Across, as a fraction of the raster's width.
        width: f64,
        /// Down, as a fraction of the raster's height.
        height: f64,
        /// How rounded the corners are, as a fraction of the shape's **own
        /// shorter side** rather than of the raster: `0` is a square corner and
        /// `0.5` is a pill, whatever size the box is and whatever shape the
        /// frame is.
        ///
        /// Relative to the shape because that is the only basis on which the
        /// number can be checked from the document alone — a raster fraction
        /// larger than the box it rounds is nonsense that nothing could catch
        /// until a render worked out how many pixels each was. It also keeps
        /// corners circular rather than elliptical, since one number then means
        /// one distance instead of two.
        #[serde(default)]
        radius: f64,
    },
    /// The ellipse that fits exactly inside a box of this size.
    ///
    /// The ellipse rather than the circle is the primitive because the two
    /// dimensions are fractions of *different* things — width of the raster's
    /// width, height of its height — so a circle on a 16:9 frame is one whose
    /// numbers account for that. Naming this `circle` and quietly picking an
    /// axis to measure both against would be a different bug on every aspect
    /// ratio.
    Ellipse {
        /// Across, as a fraction of the raster's width.
        width: f64,
        /// Down, as a fraction of the raster's height.
        height: f64,
    },
}

impl Geometry {
    /// How far across the raster the shape reaches, as a fraction of its width.
    pub fn width(self) -> f64 {
        match self {
            Self::Rectangle { width, .. } | Self::Ellipse { width, .. } => width,
        }
    }

    /// How far down the raster the shape reaches, as a fraction of its height.
    pub fn height(self) -> f64 {
        match self {
            Self::Rectangle { height, .. } | Self::Ellipse { height, .. } => height,
        }
    }

    /// How rounded the corners are. An ellipse is all corner, so the question
    /// does not arise and the answer is the one that changes nothing.
    pub fn radius(self) -> f64 {
        match self {
            Self::Rectangle { radius, .. } => radius,
            Self::Ellipse { .. } => 0.0,
        }
    }
}
