//! What an area is painted: one colour, or a gradient between several.
//!
//! A `fill` used to be a colour string and nothing else, and it still is one
//! wherever a document says `"#0b1020"` — that form parses exactly as it always
//! has. What is new is the object form beside it:
//!
//! ```json
//! { "linear": { "angle": 180, "stops": [["#0b1020", 0.0], ["#1b2440", 1.0]] } }
//! { "radial": { "center": { "x": 0.5, "y": 0.45 }, "radius": 0.8, "stops": [...] } }
//! ```
//!
//! **Why it belongs.** A flat full-frame colour is what makes an explainer
//! look like a slide deck; a radial gradient lighter behind the subject is the
//! default backdrop of every motion-graphics template, and a gradient panel is
//! the standard caption plate. It is one of the most ordinary controls in any
//! approachable editor, and conic or mesh gradients — the professional end —
//! are deliberately not here.
//!
//! **Every coordinate is a fraction of the painted area's own box**: the
//! shape's box for a shape, the whole raster for a `color` asset. So a
//! gradient pill looks the same wherever it is placed, and the same document
//! reads the same at every render resolution.
//!
//! **The generality rule holds**: core says a fill may be a gradient with
//! these stops; it never says which. Not animatable — the stops are static,
//! and a shape that fades does it through `opacity` like every other layer.

use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::color::Rgba;
use crate::shape::Point;

/// What an area is painted.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    /// One colour, written as the plain `#rrggbb` string a fill always was.
    Solid(Rgba),
    /// A gradient along a straight line across the box.
    Linear(Linear),
    /// A gradient outwards from a point, in circles.
    Radial(Radial),
}

impl Fill {
    /// The one colour this is, or `None` for a gradient — for the places that
    /// can only take one colour and have to say so rather than pick a stop.
    pub fn solid(&self) -> Option<Rgba> {
        match self {
            Self::Solid(color) => Some(*color),
            Self::Linear(_) | Self::Radial(_) => None,
        }
    }

    /// A gradient's stops, and none for a solid colour.
    pub fn stops(&self) -> &[Stop] {
        match self {
            Self::Solid(_) => &[],
            Self::Linear(linear) => &linear.stops,
            Self::Radial(radial) => &radial.stops,
        }
    }
}

impl From<Rgba> for Fill {
    fn from(color: Rgba) -> Self {
        Self::Solid(color)
    }
}

/// A gradient along a line, the way CSS's `linear-gradient` draws one.
///
/// **The CSS convention, on purpose**, since it is the one anybody who has
/// written a gradient already knows: `angle` is in degrees, clockwise, and
/// names the direction the gradient *travels* — `0` runs bottom to top, `90`
/// left to right, `180` top to bottom. The line passes through the box's
/// centre and is exactly long enough that the first stop lands on one corner
/// and the last on the opposite one, so nothing past either end is left
/// unpainted by a colour somebody chose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Linear {
    /// Which way the colours travel, in degrees clockwise from "upwards".
    /// Absent is `180`, top to bottom — CSS's default and a backdrop's.
    #[serde(default = "default_angle")]
    pub angle: f64,
    /// The colours, and where along the line each one is.
    pub stops: Vec<Stop>,
}

fn default_angle() -> f64 {
    180.0
}

/// A gradient in circles around a point.
///
/// **`radius` is a fraction of the box's shorter side** — the unit a
/// rectangle's corner `radius` already uses, and for its reason: one number
/// becomes one distance, so the circles stay circles on a box of any
/// proportion. `0.5` from the centre just touches the nearer pair of edges;
/// on a 16:9 frame about `1.02` reaches the corners. Past the radius, the last
/// stop's colour carries on to the edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Radial {
    /// Where the circles are centred, as fractions of the box: `x` across from
    /// its left edge, `y` down from its top. Absent is the middle.
    #[serde(default = "middle")]
    pub center: Point,
    /// How far the last stop is from the centre.
    pub radius: f64,
    /// The colours, from the centre outwards.
    pub stops: Vec<Stop>,
}

fn middle() -> Point {
    Point::new(0.5, 0.5)
}

/// One colour of a gradient and where it sits: `0` the start, `1` the end.
///
/// Written as a pair — `["#0b1020", 0.0]` — because that is how the stop reads
/// in every gradient editor, and a field name per number is noise in a list of
/// them. Two stops at the same place are a hard edge, as in CSS.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "(Rgba, f64)", into = "(Rgba, f64)")]
pub struct Stop {
    /// The colour at this point, alpha included.
    pub color: Rgba,
    /// Where along the gradient, from `0` to `1`.
    pub at: f64,
}

impl Stop {
    /// A stop of this colour at this point.
    pub const fn new(color: Rgba, at: f64) -> Self {
        Self { color, at }
    }
}

impl From<(Rgba, f64)> for Stop {
    fn from((color, at): (Rgba, f64)) -> Self {
        Self { color, at }
    }
}

impl From<Stop> for (Rgba, f64) {
    fn from(stop: Stop) -> Self {
        (stop.color, stop.at)
    }
}

/// The object form, one key naming which gradient. Its own type so serde
/// writes the error for an unknown key or a misspelt field, which is the
/// message an author needs.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Gradient {
    Linear(Linear),
    Radial(Radial),
}

impl Serialize for Fill {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Solid(color) => color.serialize(serializer),
            Self::Linear(linear) => Gradient::Linear(linear.clone()).serialize(serializer),
            Self::Radial(radial) => Gradient::Radial(radial.clone()).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Fill {
    /// A string is a colour, read exactly as it always was; an object is a
    /// gradient. Told apart by what arrived rather than by trying one and then
    /// the other, so each form's refusal is its own and says what was wrong.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(FillVisitor)
    }
}

struct FillVisitor;

impl<'de> Visitor<'de> for FillVisitor {
    type Value = Fill;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "a colour like \"#0b1020\", or a gradient: { \"linear\": { … } } or { \"radial\": { … } }",
        )
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Fill, E> {
        text.parse().map(Fill::Solid).map_err(E::custom)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Fill, A::Error> {
        let gradient = Gradient::deserialize(de::value::MapAccessDeserializer::new(map))?;
        Ok(match gradient {
            Gradient::Linear(linear) => Fill::Linear(linear),
            Gradient::Radial(radial) => Fill::Radial(radial),
        })
    }
}

impl fmt::Display for Fill {
    /// A colour as its hex; a gradient as its kind and its colours in order —
    /// what a description of a shot or an edit's *was → now* needs to say.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            Self::Solid(color) => return write!(f, "{color}"),
            Self::Linear(linear) => format!("linear {}°", linear.angle),
            Self::Radial(_) => "radial".to_owned(),
        };
        let colors: Vec<String> = self
            .stops()
            .iter()
            .map(|stop| stop.color.to_string())
            .collect();
        write!(f, "{kind} gradient {}", colors.join(" → "))
    }
}
