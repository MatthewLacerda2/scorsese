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
    ///
    /// **An arrow is judged on its stroke alone.** A line has no interior, so a
    /// `fill` on one is read by nothing — an arrow with a fill and no stroke
    /// would come out as an empty frame, which is precisely the case this
    /// answers `false` for.
    pub fn draws(&self) -> bool {
        if !self.geometry.is_closed() {
            return self.stroke.is_some();
        }
        self.fill.is_some() || self.stroke.is_some()
    }
}

/// The outline a shape has.
///
/// Three of them, and the list is meant to stay short. A rectangle, an ellipse
/// and an arrow are what a diagram is made of — boxes are its nouns and arrows
/// its verbs; polygons, stars and arbitrary paths are a drawing program growing
/// inside a video editor.
///
/// **The arrow is a different shape of thing from the other two**, and the
/// difference runs all the way through. A closed shape has a size and is placed
/// by `anchor` and `transform.position` like every other layer; an arrow has
/// two endpoints that say where it starts and ends outright. That is why it is
/// a variant here rather than another pair of numbers: no size to anchor, no
/// interior to fill.
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
    /// A stroked line from one point to another, with a head on it.
    ///
    /// A plain connecting line is this with no head, rather than a variant of
    /// its own: the two would be the same geometry, the same path and the same
    /// stroke, differing in whether one triangle is filled at the end.
    Arrow {
        /// Where it starts.
        from: Point,
        /// Where it ends, and where the head goes.
        to: Point,
        /// Straight, or bowed into an S.
        #[serde(default)]
        curve: Curve,
        /// Which ends carry a head. Absent means the end alone, which is what
        /// an arrow drawn between two boxes almost always wants.
        #[serde(default)]
        heads: Heads,
    },
}

impl Geometry {
    /// The box a closed shape is inscribed in, as fractions of the raster —
    /// width against its width, height against its height.
    ///
    /// `None` for an arrow, and that is the point of the method: an arrow has
    /// no size, and a caller reaching for one has to say what it means to do
    /// about that rather than being handed a zero or a bounding box nobody
    /// wrote.
    pub fn size(self) -> Option<(f64, f64)> {
        match self {
            Self::Rectangle { width, height, .. } | Self::Ellipse { width, height } => {
                Some((width, height))
            }
            Self::Arrow { .. } => None,
        }
    }

    /// How rounded the corners are. An ellipse is all corner and an arrow has
    /// none, so for both the question does not arise and the answer is the one
    /// that changes nothing.
    pub fn radius(self) -> f64 {
        match self {
            Self::Rectangle { radius, .. } => radius,
            Self::Ellipse { .. } | Self::Arrow { .. } => 0.0,
        }
    }

    /// True for the shapes with an inside — the ones a `fill` means anything
    /// to.
    ///
    /// An arrow is a line, and a line encloses nothing. This is what makes a
    /// `fill` on one a refusal rather than a field quietly ignored.
    pub fn is_closed(self) -> bool {
        !matches!(self, Self::Arrow { .. })
    }
}

/// One end of an arrow, as fractions of the raster.
///
/// **Absolute, not an offset.** `x` is a fraction of the frame's width measured
/// from its left edge and `y` a fraction of its height from the top, so
/// `{ "x": 0.5, "y": 0.5 }` is the middle of the picture whatever size it is
/// rendered at. That is the one place these fractions differ from
/// `transform.position`, which offsets a layer from where it already sits —
/// a line has no "already sits" to offset from.
///
/// A point outside `0`–`1` is allowed and is not a mistake: an arrow entering
/// from off-screen is an ordinary thing to draw.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    /// Across, from the left edge.
    pub x: f64,
    /// Down, from the top edge.
    pub y: f64,
}

impl Point {
    /// A point at the given fractions.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Whether both coordinates are numbers a frame could be drawn against.
    pub fn is_placed(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// How an arrow gets from one end to the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Curve {
    /// The shortest way. The default, and right whenever the two ends face each
    /// other.
    #[default]
    Straight,
    /// Bowed into an S: it leaves the start and arrives at the end along the
    /// same axis, which is the shape a connector between two boxes side by side
    /// wants. A straight diagonal between them reads as a mistake.
    ///
    /// Which axis it leaves along is not a field — see
    /// [`crate::shape::Geometry::Arrow`] and the drawing code, which infer it
    /// from the run. It is not a question an author has an opinion about.
    S,
}

/// Which ends of an arrow carry a head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Heads {
    /// A plain connecting line. This is what makes `line` not a variant of its
    /// own: it is an arrow that points at nothing.
    None,
    /// A head at `to` alone. The default, because an arrow is drawn to say
    /// *this leads to that*, and that reading has a direction.
    #[default]
    End,
    /// A head at both ends — *these two are connected*, without a direction.
    Both,
}

impl Heads {
    /// Whether the `to` end is pointed.
    pub fn at_end(self) -> bool {
        matches!(self, Self::End | Self::Both)
    }

    /// Whether the `from` end is pointed.
    pub fn at_start(self) -> bool {
        matches!(self, Self::Both)
    }
}
