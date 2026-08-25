//! Writing down the assets nothing brings in, and the lanes they sit on.
//!
//! Four asset kinds have no file behind them — a caption, a colour card, a box,
//! a symbol — and until these verbs existed the only way to add one was to send
//! the whole `project.json` back. On a captioned cut that is tens of kilobytes
//! per line of text, which is how a real session came to bypass the guarded
//! path altogether and overwrite work `synth_bake` had just written.
//!
//! **One verb per kind, and that is the self-describing rule deciding it.**
//! What a kind *requires* differs by kind: a colour asset must have a colour, a
//! symbol must have a name and a size. A single `asset_new` taking a `kind` and
//! a free-form block could not say any of that in its schema — the block would
//! be one undescribed object, which is exactly the thing `tests/described.rs`
//! exists to refuse. `synth_new` is the same shape one kind further along.
//!
//! **`asset_set` is one verb, for the mirror-image reason.** It requires
//! nothing but the asset, so every field on it is optional by construction and
//! each one still describes itself and says which kinds it belongs to. Four
//! set-verbs would be four schemas restating the same eight adjectives.

mod color;
mod icon;
mod set;
mod shape;
mod text;
mod track;

pub(crate) use color::ColorNew;
pub(crate) use icon::IconNew;
pub(crate) use set::AssetSet;
pub(crate) use shape::ShapeNew;
pub(crate) use text::TextNew;
pub(crate) use track::TrackNew;

use scorsese_core::{AuthorError, Project, Rgba, TextAlign, TextStyle};
use serde_json::{Map, Value};

/// A required string argument, and what it is meant to say.
fn words(arguments: &Value, key: &str, what: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("`{key}` is required: {what}"))
}

/// An optional string argument, absent when it is blank — which is also how
/// the id a caller asked the new thing to be called is read.
fn maybe(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// A number argument, refused rather than rounded when it is not one.
///
/// An infinity or a NaN is refused here rather than written: both serialise as
/// `null`, so a document that took one would come back a different document.
fn number(arguments: &Value, key: &str) -> Result<Option<f64>, String> {
    let Some(value) = arguments.get(key).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_f64()
        .filter(|number| number.is_finite())
        .map(Some)
        .ok_or_else(|| format!("`{key}` has to be a number"))
}

/// The same, required.
fn required_number(arguments: &Value, key: &str, what: &str) -> Result<f64, String> {
    number(arguments, key)?.ok_or_else(|| format!("`{key}` is required: {what}"))
}

/// A whole number in OpenType's `wght` range — anything else is not a weight.
fn weight(arguments: &Value) -> Result<Option<u16>, String> {
    let Some(value) = arguments.get("weight").filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_u64()
        .and_then(|number| u16::try_from(number).ok())
        .map(Some)
        .ok_or_else(|| "`weight` has to be a whole number, 1 to 1000".to_owned())
}

/// A colour, read the way the document writes one.
fn color(arguments: &Value, key: &str) -> Result<Option<Rgba>, String> {
    let Some(text) = maybe(arguments, key) else {
        return Ok(None);
    };
    text.parse()
        .map(Some)
        .map_err(|problem| format!("`{key}`: {problem}"))
}

/// The same, required — for the kinds that have no colour it would be safe to
/// invent.
fn required_color(arguments: &Value, key: &str, what: &str) -> Result<Rgba, String> {
    color(arguments, key)?.ok_or_else(|| format!("`{key}` is required: {what}"))
}

/// Which edge the lines line up against.
fn align(arguments: &Value) -> Result<Option<TextAlign>, String> {
    match maybe(arguments, "align").as_deref() {
        None => Ok(None),
        Some("left") => Ok(Some(TextAlign::Left)),
        Some("center") => Ok(Some(TextAlign::Center)),
        Some("right") => Ok(Some(TextAlign::Right)),
        Some(other) => Err(format!("`align` is left, center or right, not `{other}`")),
    }
}

/// The look a new caption is set in, or `None` when nothing about it was said
/// — which leaves the style out of the document rather than writing the
/// defaults into it.
fn style(arguments: &Value) -> Result<Option<TextStyle>, String> {
    let mut style = TextStyle::default();
    let mut said = false;
    if let Some(font) = maybe(arguments, "font") {
        style.font = font.into();
        said = true;
    }
    if let Some(weight) = weight(arguments)? {
        style.weight = Some(weight);
        said = true;
    }
    if let Some(italic) = arguments.get("italic").and_then(Value::as_bool) {
        style.italic = italic;
        said = true;
    }
    for (value, field) in [
        (number(arguments, "size")?, &mut style.size),
        (number(arguments, "line_height")?, &mut style.line_height),
        (number(arguments, "max_width")?, &mut style.max_width),
    ] {
        if let Some(value) = value {
            *field = value;
            said = true;
        }
    }
    if let Some(color) = color(arguments, "color")? {
        style.color = color;
        said = true;
    }
    if let Some(align) = align(arguments)? {
        style.align = align;
        said = true;
    }
    if let Some(stroke) = color(arguments, "stroke")? {
        style.stroke = Some(stroke);
        said = true;
    }
    if let Some(width) = number(arguments, "stroke_width")? {
        style.stroke_width = width;
        said = true;
    }
    Ok(said.then_some(style))
}

/// How a refusal from the model reads on the wire: the reason, and the promise
/// that goes with every one of them.
fn refused(error: AuthorError) -> String {
    format!("{error} — nothing was written")
}

/// Writes the document back, or says why it could not.
fn save(project: &Project, dir: &std::path::Path) -> Result<(), String> {
    project
        .save(dir)
        .map_err(|error| format!("saving the project: {error}"))
}

/// The appearance fields, described once wherever they appear.
///
/// A `size` means the same thing on a caption and on a symbol, and a `color`
/// is written the same way on all four kinds — so two descriptions of either
/// would be two chances to drift. Each tool asks for the ones its kind takes.
fn described(field: &str) -> Value {
    match field {
        "font" => serde_json::json!({ "type": "string",
            "description": "The face: a name this build ships — `sans`, `serif` — or a \
                            path to a font file inside the project." }),
        "weight" => serde_json::json!({ "type": "integer",
            "description": "How heavy the glyphs are, 1 to 1000 on the usual scale where \
                            400 is regular and 700 bold. Read from a variable font only." }),
        "italic" => serde_json::json!({ "type": "boolean",
            "description": "Set it in the family's italic — a different drawing, not the \
                            upright leaned over. Only for a face this build ships." }),
        "size" => serde_json::json!({ "type": "number",
            "description": "How big, as a fraction of the frame's HEIGHT: 0.1 is a tenth \
                            of the picture. Means the same at every render resolution." }),
        "color" => serde_json::json!({ "type": "string",
            "description": "The colour, as `#rrggbb` — or `#rrggbbaa` for one you can see \
                            through, which composites over whatever is under it." }),
        "align" => serde_json::json!({ "type": "string",
            "enum": ["left", "center", "right"],
            "description": "Which edge the lines line up against inside the wrapped \
                            block. Default `center`, which is what a title wants." }),
        "line_height" => serde_json::json!({ "type": "number",
            "description": "Baseline to baseline, as a multiple of `size`. 1.0 sets the \
                            lines solid; the default 1.25 leaves a readable gap." }),
        "max_width" => serde_json::json!({ "type": "number",
            "description": "How wide the text runs before it wraps, as a fraction of the \
                            frame's WIDTH. Default 0.9, a margin down each side." }),
        "fill" => serde_json::json!({ "type": "string",
            "description": "What the inside of the shape is painted, as `#rrggbb`. Leave \
                            it out for a see-through middle — a callout over footage." }),
        "stroke" => serde_json::json!({ "type": "string",
            "description": "The rim, as `#rrggbb`. On a shape it is the border; on a \
                            caption it is an outline added OUTSIDE the letterform, \
                            which is what keeps burned-in words legible over footage. \
                            Left out, there is no rim at all." }),
        "stroke_width" => serde_json::json!({ "type": "number",
            "description": "How thick that rim is. On a shape or a caption, a fraction \
                            of the frame's height (0.004 and 0.002 by default); on an \
                            icon, a fraction of the icon's own box, so it scales with \
                            the symbol." }),
        "width" => serde_json::json!({ "type": "number",
            "description": "Across, as a fraction of the frame's width. A closed shape \
                            only — an arrow is its two endpoints." }),
        "height" => serde_json::json!({ "type": "number",
            "description": "Down, as a fraction of the frame's height. A closed shape \
                            only." }),
        "radius" => serde_json::json!({ "type": "number",
            "description": "How rounded a rectangle's corners are, as a fraction of its \
                            own shorter side: 0 is square and 0.5 a pill." }),
        other => serde_json::json!({ "type": "string",
            "description": format!("`{other}` — undescribed, which is a bug in this build") }),
    }
}

/// The named appearance fields as a schema's `properties`, alongside the
/// project every tool takes.
fn properties(fields: &[&str]) -> Map<String, Value> {
    let mut properties = Map::new();
    properties.insert("project".to_owned(), super::project_property());
    for field in fields {
        properties.insert((*field).to_owned(), described(field));
    }
    properties
}

/// The `id to call it` argument, worded once for the four kinds that share it.
fn id_property(what: &str) -> Value {
    serde_json::json!({
        "type": "string",
        "description": format!(
            "What to call the new asset. Optional: without it an id is derived from {what} \
             and suffixed until it is free, and the reply says which one it wrote. An id \
             already in use is refused rather than quietly changed, because you are about \
             to write it onto a clip."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every property a schema carries has to describe itself — that is the
    /// gate `tests/described.rs` walks. A field name nobody wrote a
    /// description for still gets one rather than an undescribed property,
    /// because a bug in this build should not turn into a capability a client
    /// cannot see.
    #[test]
    fn even_an_unknown_field_describes_itself() {
        let described = described("nonsense");
        let said = described["description"].as_str().expect("a description");
        assert!(said.contains("nonsense"), "got {said}");
    }

    /// The shared descriptions are the reason this table exists: a `size` says
    /// the same thing wherever it is asked for.
    #[test]
    fn the_project_comes_first_and_every_named_field_follows() {
        let properties = properties(&["size", "color"]);
        assert!(properties.contains_key("project"));
        assert_eq!(properties.len(), 3);
        for field in ["size", "color"] {
            assert!(
                properties[field]["description"].is_string(),
                "`{field}` says nothing about itself"
            );
        }
    }
}
