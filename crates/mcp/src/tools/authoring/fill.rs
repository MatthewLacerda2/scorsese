//! A colour or a gradient, as a tool argument: the string a colour always was,
//! or the object `project.json` writes a gradient as.
//!
//! The object is read by the same deserialiser the document uses, so a tool
//! cannot accept a gradient the format would refuse or spell one differently;
//! whether its stops make sense is validation's to say, before anything is
//! written.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use scorsese_core::Fill;
use serde::Deserialize;
use serde_json::Value;

/// How the object form reads, shared by every argument that takes one.
const GRADIENT: &str = "Or a gradient, as an object — {\"linear\": {\"angle\": 180, \
    \"stops\": [[\"#0b1020\", 0], [\"#1b2440\", 1]]}} runs top to bottom (angle in degrees \
    as in CSS: 0 upwards, 90 left to right, 180 downwards; default 180), and \
    {\"radial\": {\"center\": {\"x\": 0.5, \"y\": 0.45}, \"radius\": 0.8, \"stops\": [...]}} \
    spreads in circles from a centre (default the middle). Each stop is [colour, offset] with \
    offsets rising from 0 to 1, at least two of them. Coordinates are fractions of the \
    painted box — the shape's own for a shape, the frame for a color asset — and `radius` \
    is a fraction of that box's SHORTER side. Gradients are dithered so a dark one does not \
    band.";

/// A fill argument as it arrived: a string or an object, read into a [`Fill`]
/// by [`fill`] so that the refusal is the document's own words for it.
///
/// Its own type so that its schema says both forms; the description is the
/// field's, since what the colour is *for* differs by argument
/// ([`described`]).
#[derive(Deserialize)]
pub(super) struct Paint(Value);

impl JsonSchema for Paint {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "Paint".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({ "type": ["string", "object"] })
    }
}

/// A fill argument, absent when it is missing, null or a blank string.
pub(super) fn fill(given: Option<&Paint>, key: &str) -> Result<Option<Fill>, String> {
    let Some(Paint(value)) = given.filter(|Paint(value)| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str().is_some_and(|text| text.trim().is_empty()) {
        return Ok(None);
    }
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|problem| format!("`{key}`: {problem}"))
}

/// The description of an argument that takes a colour or a gradient, `lead`
/// being what the argument is for.
pub(super) fn described(lead: &str) -> String {
    format!("{lead} {GRADIENT}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::Rgba;

    fn paint(value: Value) -> Paint {
        Paint(value)
    }

    #[test]
    fn a_string_is_the_colour_it_always_was() {
        let read = fill(Some(&paint(serde_json::json!("#ff0000"))), "fill").expect("a colour");
        assert_eq!(read, Some(Fill::Solid(Rgba::opaque(0xff, 0, 0))));
        assert_eq!(fill(Some(&paint(serde_json::json!(" "))), "fill"), Ok(None));
    }

    #[test]
    fn an_object_is_a_gradient_and_a_misspelling_is_named() {
        let given = paint(serde_json::json!({ "radial": {
            "radius": 0.5, "stops": [["#000000", 0], ["#ffffff", 1]] } }));
        assert!(matches!(
            fill(Some(&given), "fill"),
            Ok(Some(Fill::Radial(_)))
        ));
        let given = paint(serde_json::json!({ "conic": {} }));
        let refused = fill(Some(&given), "fill").expect_err("no conic gradients");
        assert!(refused.contains("conic"), "{refused}");
    }
}
