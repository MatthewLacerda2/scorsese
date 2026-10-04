//! A box, an ellipse or an arrow, drawn by the render rather than imported.

use schemars::{JsonSchema, Schema, SchemaGenerator};
use scorsese_core::{Curve, Geometry, Heads, Inline, Shape, authoring};
use serde::Deserialize;
use serde_json::Value;

use super::fill::{self, Paint, fill};
use super::{
    FILL, HEIGHT, RADIUS, STROKE, STROKE_WIDTH, WIDTH, color, id_described, maybe, refused, save,
};
use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

mod arrow;

/// Add a `shape` asset.
pub(crate) struct ShapeNew;

/// Which outline, as the call names it.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Outline {
    Rectangle,
    Ellipse,
    Arrow,
}

/// What `shape_new` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Which outline. `rectangle` and `ellipse` need a width and a height;
    /// `arrow` needs `from` and `to` and has no size of its own.
    geometry: Outline,
    #[schemars(description = WIDTH)]
    width: Option<f64>,
    #[schemars(description = HEIGHT)]
    height: Option<f64>,
    #[schemars(description = RADIUS)]
    radius: Option<f64>,
    #[schemars(description = fill::described(FILL))]
    fill: Option<Paint>,
    #[schemars(description = STROKE)]
    stroke: Option<String>,
    #[schemars(description = STROKE_WIDTH)]
    stroke_width: Option<f64>,
    #[serde(default)]
    #[schemars(with = "arrow::End", description = arrow::endpoint_described("starts"))]
    from: Option<arrow::End>,
    #[serde(default)]
    #[schemars(with = "arrow::End", description = arrow::endpoint_described("ends, head first"))]
    to: Option<arrow::End>,
    // Described by its schema, which also bounds each length.
    #[serde(default)]
    #[schemars(schema_with = "dash_schema")]
    dash: Option<Vec<f64>>,
    /// How the arrow gets from one end to the other — an `arrow` only.
    /// `straight` is the default; `s` bows it so it leaves and arrives along
    /// the same axis, which is what a connector between two boxes side by side
    /// wants.
    #[serde(default)]
    #[schemars(with = "arrow::Line")]
    curve: Option<arrow::Line>,
    /// Which ends carry a head — an `arrow` only. `end` is the default and
    /// points at `to`; `none` is a plain connecting line; `both` says these two
    /// are connected, without a direction.
    #[serde(default)]
    #[schemars(with = "arrow::Tips")]
    heads: Option<arrow::Tips>,
    #[schemars(description = id_described("the outline"))]
    asset: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("geometry", "rectangle, ellipse or arrow")];
}

impl Tool for ShapeNew {
    fn name(&self) -> &'static str {
        "shape_new"
    }

    fn description(&self) -> &'static str {
        "Add a shape asset: a rectangle, an ellipse or an arrow, drawn by the \
         render rather than imported as a picture of one. A panel behind a \
         caption, a ring around a face, a connector between two boxes on a \
         diagram. Everything about it is a fraction of the frame, so one \
         document reads the same at 640x360 and at 4K and the edges stay clean \
         instead of stepping. A rectangle and an ellipse take a width and a \
         height; an arrow takes two endpoints instead, each either a point on \
         the frame or a clip to follow — an attached end is resolved on every \
         frame, so the arrow moves when the box it points at does. A shape with \
         neither a fill nor a border draws nothing and is refused, because a \
         layer that renders nothing looks exactly like one that failed to. \
         The fill can be a gradient laid across the shape's own box — a \
         gradient pill is the standard caption plate. \
         The border can be dashed (`dash`); its dashes made to flow by \
         keyframing `shape.dash_offset` on the clip, and the line made to draw \
         itself on by keyframing `shape.trim_end` from 0 to 1 — an arrow's \
         head rides the drawn end."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let geometry = arguments.geometry()?;
        let outline = say(&geometry);
        // Whether the lengths are ones a line can be broken into is
        // validation's to say, so an empty list or a zero is passed through
        // and refused there, in the same words a hand-written document gets.
        let shape = Shape {
            geometry,
            fill: fill(arguments.fill.as_ref(), "fill")?,
            stroke: color(arguments.stroke.as_deref(), "stroke")?,
            stroke_width: arguments
                .stroke_width
                .unwrap_or(scorsese_core::DEFAULT_STROKE_WIDTH),
            dash: arguments.dash.clone(),
        };
        let id = authoring::add_asset(
            &mut project,
            maybe(arguments.asset.as_deref()).as_deref(),
            Inline::Shape(shape),
        )
        .map_err(refused)?;
        save(&project, dir)?;
        Ok(format!(
            "`{id}` — a shape asset, {outline}. place_clip puts it on a video \
             track, with a duration."
        )
        .into())
    }
}

impl Arguments {
    /// The outline the arguments describe.
    fn geometry(&self) -> Result<Geometry, String> {
        let sized = |what: &str| -> Result<(f64, f64), String> {
            let needed = |value: Option<f64>, field: &str, how: &str| {
                value.ok_or_else(|| format!("`{field}` is required: how {how} the {what} is"))
            };
            Ok((
                needed(self.width, "width", "wide")?,
                needed(self.height, "height", "tall")?,
            ))
        };
        match self.geometry {
            Outline::Rectangle => {
                let (width, height) = sized("box")?;
                Ok(Geometry::Rectangle {
                    width,
                    height,
                    radius: self.radius.unwrap_or_default(),
                })
            }
            Outline::Ellipse => {
                let (width, height) = sized("ellipse")?;
                Ok(Geometry::Ellipse { width, height })
            }
            Outline::Arrow => Ok(Geometry::Arrow {
                from: arrow::endpoint(self.from.as_ref(), "from")?,
                to: arrow::endpoint(self.to.as_ref(), "to")?,
                curve: self.curve.map_or(Curve::Straight, Into::into),
                heads: self.heads.map_or(Heads::End, Into::into),
            }),
        }
    }
}

/// The `dash` argument's schema: a list of lengths, each above zero.
fn dash_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({
        "type": "array",
        "items": { "type": "number", "exclusiveMinimum": 0 },
        "minItems": 1,
        "description": "Break the border into dashes: lengths along the line, \
                        on, off, on, off…, each a fraction of the frame's height \
                        like `stroke_width` — `[0.02, 0.012]` is a dash twice as \
                        long as the line is thick at the default width, then a \
                        gap. An odd count is read twice over, so `[0.02]` is \
                        dashes and gaps of one length. Absent is a solid line."
    })
}

/// How the outline reads back, so a caller can see what it wrote.
fn say(geometry: &Geometry) -> String {
    match geometry {
        Geometry::Rectangle { width, height, .. } => format!("a {width}x{height} rectangle"),
        Geometry::Ellipse { width, height } => format!("a {width}x{height} ellipse"),
        Geometry::Arrow { .. } => "an arrow".to_owned(),
    }
}
