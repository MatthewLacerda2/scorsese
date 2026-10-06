//! Making a box, an ellipse or an arrow, drawn by the render rather than
//! imported as a picture of one.

use schemars::{JsonSchema, Schema, SchemaGenerator};
use scorsese_core::{Curve, Geometry, Heads, Inline, Shape};
use serde::Deserialize;

use super::super::color;
use super::super::fill::fill;
use super::{Arguments, arrow};

/// Which outline, as the call names it.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(super) enum Outline {
    Rectangle,
    Ellipse,
    Arrow,
}

/// The shape asset the arguments describe, and how its outline reads back so
/// a caller can see what it wrote.
pub(super) fn made(arguments: &Arguments) -> Result<(Inline, String), String> {
    let geometry = geometry(arguments)?;
    let outline = say(&geometry);
    // Whether the lengths are ones a line can be broken into is validation's
    // to say, so an empty list or a zero is passed through and refused there,
    // in the same words a hand-written document gets.
    let shape = Shape {
        geometry,
        fill: fill(arguments.fill.as_ref(), "fill")?,
        stroke: color(arguments.stroke.as_deref(), "stroke")?,
        stroke_width: arguments
            .stroke_width
            .unwrap_or(scorsese_core::DEFAULT_STROKE_WIDTH),
        dash: arguments.dash.clone(),
    };
    Ok((Inline::Shape(shape), outline))
}

/// The outline the arguments describe.
fn geometry(arguments: &Arguments) -> Result<Geometry, String> {
    let outline = arguments
        .geometry
        .ok_or("`geometry` is required: rectangle, ellipse or arrow")?;
    let sized = |what: &str| -> Result<(f64, f64), String> {
        let needed = |value: Option<f64>, field: &str, how: &str| {
            value.ok_or_else(|| format!("`{field}` is required: how {how} the {what} is"))
        };
        Ok((
            needed(arguments.width, "width", "wide")?,
            needed(arguments.height, "height", "tall")?,
        ))
    };
    match outline {
        Outline::Rectangle => {
            let (width, height) = sized("box")?;
            Ok(Geometry::Rectangle {
                width,
                height,
                radius: arguments.radius.unwrap_or_default(),
            })
        }
        Outline::Ellipse => {
            let (width, height) = sized("ellipse")?;
            Ok(Geometry::Ellipse { width, height })
        }
        Outline::Arrow => Ok(Geometry::Arrow {
            from: arrow::endpoint(arguments.from.as_ref(), "from")?,
            to: arrow::endpoint(arguments.to.as_ref(), "to")?,
            curve: arguments.curve.map_or(Curve::Straight, Into::into),
            heads: arguments.heads.map_or(Heads::End, Into::into),
        }),
    }
}

/// The `dash` argument's schema: a list of lengths, each above zero.
pub(super) fn dash_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({
        "type": "array",
        "items": { "type": "number", "exclusiveMinimum": 0 },
        "minItems": 1,
        "description": "(shape, when making it) Break the border into dashes: \
                        lengths along the line, on, off, on, off…, each a fraction \
                        of the frame's height like `stroke_width` — `[0.02, 0.012]` \
                        is a dash twice as long as the line is thick at the default \
                        width, then a gap. An odd count is read twice over. Absent \
                        is a solid line. Keyframing `shape.dash_offset` on the clip \
                        makes the dashes flow, and `shape.trim_end` from 0 to 1 \
                        draws the line on — an arrow's head rides the drawn end."
    })
}

/// How the outline reads back.
fn say(geometry: &Geometry) -> String {
    match geometry {
        Geometry::Rectangle { width, height, .. } => format!("a {width}x{height} rectangle"),
        Geometry::Ellipse { width, height } => format!("a {width}x{height} ellipse"),
        Geometry::Arrow { .. } => "an arrow".to_owned(),
    }
}
