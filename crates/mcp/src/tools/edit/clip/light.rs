//! A clip's light of its own over the wire: `shadow`, `glow` and `blend`.
//!
//! **An object merges; `false` removes.** Every other argument of `clip_set`
//! leaves what it does not name alone, and these keep that promise one level
//! down: `"glow": { "intensity": 2 }` on a clip that already glows brightens
//! it and keeps its colour and reach, and on a clip that does not it starts
//! from the defaults a document's `"glow": {}` would. `false` is the one way
//! to take either away, because `null` already means "not given" here.

use scorsese_core::{Blend, Clip, Glow, Rgba, Shadow};
use serde_json::{Map, Value, json};

/// The three arguments' schemas, for `clip_set`'s own.
pub(super) fn schema() -> [(&'static str, Value); 3] {
    let colour = |what: &str| {
        json!({
            "type": "string",
            "description": format!("{what}, as `#rrggbb` or `#rrggbbaa`.")
        })
    };
    let number = |description: &str| json!({ "type": "number", "description": description });
    [
        (
            "shadow",
            json!({
                "description": "A drop shadow: the clip's own silhouette, softened, offset and \
                                drawn under it. An object sets the fields it names and keeps \
                                the rest (the defaults, on a clip without one: black, 0.01 \
                                down and right, softness 0.02, opacity 0.5); `false` removes \
                                it. Lengths are fractions of the clip's own height, as `blur` \
                                is. Picture only.",
                "type": ["object", "boolean"],
                "properties": {
                    "color": colour("The shadow's colour; its alpha scales the opacity"),
                    "offset_x": number("How far right it falls, as a fraction of the \
                                        clip's height; negative is left."),
                    "offset_y": number("How far down it falls, as a fraction of the \
                                        clip's height; negative is up."),
                    "softness": number("How soft its edge is, measured as `blur` is. \
                                        0 is hard."),
                    "opacity": number("How dark it is, 0 to 1.")
                },
                "additionalProperties": false
            }),
        ),
        (
            "glow",
            json!({
                "description": "A soft halo of light round whatever the clip draws, drawn \
                                under it. An object sets the fields it names and keeps the \
                                rest (the defaults, on a clip without one: the clip's own \
                                colours, radius 0.02, intensity 1); `false` removes it. On a \
                                group clip it lights the whole group. Picture only.",
                "type": ["object", "boolean"],
                "properties": {
                    "color": colour("A colour for the light, or `own` for the \
                                     clip's own colours"),
                    "radius": number("How far the halo reaches, as a fraction of the \
                                      clip's height, measured as `blur` is."),
                    "intensity": number("How bright: 0 none, 1 the clip's own light \
                                         spread out, up to 4 for thin lines.")
                },
                "additionalProperties": false
            }),
        ),
        (
            "blend",
            json!({
                "type": "string",
                "enum": ["normal", "add", "screen", "multiply"],
                "description": "How the clip lands on what is beneath it, shadow and glow \
                                included: `normal` covers; `add` and `screen` add light, so \
                                overlapping glowing things brighten; `multiply` darkens. \
                                Over nothing but the black frame, `add` and `screen` look \
                                exactly like `normal`. Picture only."
            }),
        ),
    ]
}

/// Applies whichever of the three `arguments` names to `clip`; what was set,
/// in words, one entry per argument.
pub(super) fn apply(clip: &mut Clip, arguments: &Value) -> Result<Vec<String>, String> {
    let mut said = Vec::new();
    if let Some(given) = given(arguments, "shadow") {
        clip.shadow = merged(given, "shadow", clip.shadow, shadow_field)?;
        said.push(match clip.shadow {
            Some(shadow) => format!(
                "shadow {} offset ({}, {}), softness {}, opacity {}",
                shadow.color, shadow.offset_x, shadow.offset_y, shadow.softness, shadow.opacity
            ),
            None => "no shadow".to_owned(),
        });
    }
    if let Some(given) = given(arguments, "glow") {
        clip.glow = merged(given, "glow", clip.glow, glow_field)?;
        said.push(match clip.glow {
            Some(glow) => format!(
                "glow in {}, radius {}, intensity {}",
                glow.color
                    .map_or_else(|| "its own colours".to_owned(), |c| c.to_string()),
                glow.radius,
                glow.intensity
            ),
            None => "no glow".to_owned(),
        });
    }
    if let Some(given) = given(arguments, "blend") {
        clip.blend = serde_json::from_value::<Blend>(given.clone()).map_err(|_| {
            format!("`blend` is `normal`, `add`, `screen` or `multiply`, not {given}")
        })?;
        said.push(format!("blend {}", clip.blend.as_str()));
    }
    Ok(said)
}

/// An argument that was given, `null` counting as not.
fn given<'a>(arguments: &'a Value, key: &str) -> Option<&'a Value> {
    arguments.get(key).filter(|value| !value.is_null())
}

/// `false` for none; an object's fields laid over what the clip had, or over
/// the defaults when it had nothing.
fn merged<T: Default>(
    given: &Value,
    key: &str,
    current: Option<T>,
    field: fn(&mut T, &str, &Value) -> Result<(), String>,
) -> Result<Option<T>, String> {
    let fields: &Map<String, Value> = match given {
        Value::Bool(false) => return Ok(None),
        Value::Object(fields) => fields,
        other => {
            return Err(format!(
                "`{key}` is an object of fields, or `false`, not {other}"
            ));
        }
    };
    let mut value = current.unwrap_or_default();
    for (name, setting) in fields {
        field(&mut value, name, setting).map_err(|problem| format!("`{key}.{name}`: {problem}"))?;
    }
    Ok(Some(value))
}

fn shadow_field(shadow: &mut Shadow, name: &str, value: &Value) -> Result<(), String> {
    match name {
        "color" => shadow.color = colour(value)?,
        "offset_x" => shadow.offset_x = number(value)?,
        "offset_y" => shadow.offset_y = number(value)?,
        "softness" => shadow.softness = number(value)?,
        "opacity" => shadow.opacity = number(value)?,
        _ => return Err("a shadow has color, offset_x, offset_y, softness and opacity".into()),
    }
    Ok(())
}

fn glow_field(glow: &mut Glow, name: &str, value: &Value) -> Result<(), String> {
    match name {
        "color" if value.as_str() == Some("own") => glow.color = None,
        "color" => glow.color = Some(colour(value)?),
        "radius" => glow.radius = number(value)?,
        "intensity" => glow.intensity = number(value)?,
        _ => return Err("a glow has color, radius and intensity".into()),
    }
    Ok(())
}

fn number(value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("has to be a number, not {value}"))
}

fn colour(value: &Value) -> Result<Rgba, String> {
    value
        .as_str()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| format!("has to be a colour like `#ffcc00`, not {value}"))
}
