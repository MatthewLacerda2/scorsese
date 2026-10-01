//! A colour or a gradient, as a tool argument: the string a colour always was,
//! or the object `project.json` writes a gradient as.
//!
//! The object is read by the same deserialiser the document uses, so a tool
//! cannot accept a gradient the format would refuse or spell one differently;
//! whether its stops make sense is validation's to say, before anything is
//! written.

use scorsese_core::Fill;
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

/// A fill argument, absent when it is missing, null or a blank string.
pub(super) fn fill(arguments: &Value, key: &str) -> Result<Option<Fill>, String> {
    let Some(value) = arguments.get(key).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str().is_some_and(|text| text.trim().is_empty()) {
        return Ok(None);
    }
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|problem| format!("`{key}`: {problem}"))
}

/// The same, required — for the colour asset, which has no colour it would be
/// safe to invent.
pub(super) fn required_fill(arguments: &Value, key: &str, what: &str) -> Result<Fill, String> {
    fill(arguments, key)?.ok_or_else(|| format!("`{key}` is required: {what}"))
}

/// The schema of an argument that takes a colour or a gradient, `lead` being
/// what the argument is for.
pub(super) fn property(lead: &str) -> Value {
    serde_json::json!({
        "type": ["string", "object"],
        "description": format!("{lead} {GRADIENT}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::Rgba;

    #[test]
    fn a_string_is_the_colour_it_always_was() {
        let arguments = serde_json::json!({ "fill": "#ff0000" });
        let read = fill(&arguments, "fill").expect("a colour");
        assert_eq!(read, Some(Fill::Solid(Rgba::opaque(0xff, 0, 0))));
    }

    #[test]
    fn an_object_is_a_gradient_and_a_misspelling_is_named() {
        let arguments = serde_json::json!({ "fill": { "radial": {
            "radius": 0.5, "stops": [["#000000", 0], ["#ffffff", 1]] } } });
        assert!(matches!(
            fill(&arguments, "fill"),
            Ok(Some(Fill::Radial(_)))
        ));
        let arguments = serde_json::json!({ "fill": { "conic": {} } });
        let refused = fill(&arguments, "fill").expect_err("no conic gradients");
        assert!(refused.contains("conic"), "{refused}");
    }
}
