//! A box, an ellipse or an arrow, drawn by the render rather than imported.

use scorsese_core::{Curve, Geometry, Heads, Inline, Shape, authoring};
use serde_json::Value;

use super::{
    color, id_property, maybe, number, properties, refused, required_number, save, wanted,
};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir};

mod arrow;

/// Add a `shape` asset.
pub(crate) struct ShapeNew;

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
         layer that renders nothing looks exactly like one that failed to."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        let mut properties = properties(&[
            "width",
            "height",
            "radius",
            "fill",
            "stroke",
            "stroke_width",
        ]);
        properties.insert(
            "geometry".to_owned(),
            serde_json::json!({
                "type": "string",
                "enum": ["rectangle", "ellipse", "arrow"],
                "description": "Which outline. `rectangle` and `ellipse` need a width \
                                and a height; `arrow` needs `from` and `to` and has no \
                                size of its own."
            }),
        );
        properties.insert("from".to_owned(), arrow::endpoint_property("starts"));
        properties.insert(
            "to".to_owned(),
            arrow::endpoint_property("ends, head first"),
        );
        properties.insert("curve".to_owned(), arrow::curve_property());
        properties.insert("heads".to_owned(), arrow::heads_property());
        properties.insert("asset".to_owned(), id_property("the outline"));
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": ["project", "geometry"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let geometry = geometry(arguments)?;
        let outline = say(&geometry);
        let shape = Shape {
            geometry,
            fill: color(arguments, "fill")?,
            stroke: color(arguments, "stroke")?,
            stroke_width: number(arguments, "stroke_width")?
                .unwrap_or(scorsese_core::DEFAULT_STROKE_WIDTH),
        };
        let id = authoring::add_asset(
            &mut project,
            wanted(arguments, "asset").as_deref(),
            Inline::Shape(shape),
        )
        .map_err(refused)?;
        save(&project, &dir)?;
        Ok(format!(
            "`{id}` — a shape asset, {outline}. place_clip puts it on a video \
             track, with a duration."
        )
        .into())
    }
}

/// The outline the arguments describe.
fn geometry(arguments: &Value) -> Result<Geometry, String> {
    let sized = |what: &str| -> Result<(f64, f64), String> {
        Ok((
            required_number(arguments, "width", &format!("how wide the {what} is"))?,
            required_number(arguments, "height", &format!("how tall the {what} is"))?,
        ))
    };
    match maybe(arguments, "geometry").as_deref() {
        Some("rectangle") => {
            let (width, height) = sized("box")?;
            Ok(Geometry::Rectangle {
                width,
                height,
                radius: number(arguments, "radius")?.unwrap_or_default(),
            })
        }
        Some("ellipse") => {
            let (width, height) = sized("ellipse")?;
            Ok(Geometry::Ellipse { width, height })
        }
        Some("arrow") => Ok(Geometry::Arrow {
            from: arrow::endpoint(arguments, "from")?,
            to: arrow::endpoint(arguments, "to")?,
            curve: arrow::curve(arguments)?.unwrap_or(Curve::Straight),
            heads: arrow::heads(arguments)?.unwrap_or(Heads::End),
        }),
        Some(other) => Err(format!(
            "`geometry` is rectangle, ellipse or arrow, not `{other}`"
        )),
        None => Err("`geometry` is required: rectangle, ellipse or arrow".to_owned()),
    }
}

/// How the outline reads back, so a caller can see what it wrote.
fn say(geometry: &Geometry) -> String {
    match geometry {
        Geometry::Rectangle { width, height, .. } => format!("a {width}x{height} rectangle"),
        Geometry::Ellipse { width, height } => format!("a {width}x{height} ellipse"),
        Geometry::Arrow { .. } => "an arrow".to_owned(),
    }
}
