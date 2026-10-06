//! Which arguments each kind takes, and the refusal for one it does not.
//!
//! One list per kind, checked before anything is read into a type, so that a
//! `fill` on a caption is refused by name whether the caption is being made or
//! changed — the `*_new` tools ignored an argument they did not declare, and an
//! argument silently dropped is an edit somebody thinks they made.

use scorsese_core::AssetKind;
use serde_json::Value;

use super::Arguments;

/// What a `text` asset takes, made or changed.
const TEXT: &[&str] = &[
    "text",
    "font",
    "weight",
    "italic",
    "size",
    "color",
    "align",
    "line_height",
    "max_width",
    "stroke",
    "stroke_width",
    "reveal",
    "number",
];

/// What a `color` asset takes, which is the whole of what it is.
const COLOR: &[&str] = &["color"];

/// What a `shape` asset takes once it exists: its paint and its size.
const SHAPE: &[&str] = &[
    "fill",
    "stroke",
    "stroke_width",
    "width",
    "height",
    "radius",
];

/// What making a shape takes besides: which outline, an arrow's ends and how
/// it runs between them, and dashes. Changing those afterwards is a different
/// outline rather than a nudge, so they are read only when it is made.
const SHAPE_MADE: &[&str] = &[
    "fill",
    "stroke",
    "stroke_width",
    "width",
    "height",
    "radius",
    "geometry",
    "from",
    "to",
    "curve",
    "heads",
    "dash",
];

/// What an `icon` asset takes.
const ICON: &[&str] = &["icon", "size", "color", "stroke_width"];

/// A shot's brief.
const VIDEO: &[&str] = &["prompt", "video"];

/// A still's brief.
const IMAGE: &[&str] = &["prompt", "image"];

/// A spoken line's brief.
const SPEECH: &[&str] = &["prompt", "speech"];

/// A synthesised asset's brief: the recipe it is baked from.
const SYNTH: &[&str] = &["recipe"];

/// The arguments `kind` takes — `making` it, or changing one that exists — or
/// `None` for a kind whose content is not in the document at all.
fn takes(kind: AssetKind, making: bool) -> Option<&'static [&'static str]> {
    match kind {
        AssetKind::Text => Some(TEXT),
        AssetKind::Color => Some(COLOR),
        AssetKind::Shape if making => Some(SHAPE_MADE),
        AssetKind::Shape => Some(SHAPE),
        AssetKind::Icon => Some(ICON),
        AssetKind::GeneratedVideo => Some(VIDEO),
        AssetKind::GeneratedImage => Some(IMAGE),
        AssetKind::GeneratedAudio => Some(SPEECH),
        AssetKind::SynthAudio => Some(SYNTH),
        _ => None,
    }
}

/// Refuses an argument `kind` has no use for, naming it and what the kind
/// does take.
pub(super) fn check(kind: AssetKind, making: bool, arguments: &Arguments) -> Result<(), String> {
    let name = kind_name(kind);
    let Some(takes) = takes(kind, making) else {
        return Err(format!(
            "a {name} asset is its file, or is written by its own tool, and has \
             nothing here to change — nothing was written"
        ));
    };
    let Some(field) = named(arguments)
        .into_iter()
        .find(|field| !takes.contains(field))
    else {
        return Ok(());
    };
    let after = if kind == AssetKind::Shape && SHAPE_MADE.contains(&field) {
        " once it is made: its outline, ends and dashes are chosen when it is"
    } else {
        ""
    };
    Err(format!(
        "`{field}` is not something a {name} asset takes{after} — it takes {}. \
         Nothing was written",
        takes.join(", ")
    ))
}

/// The arguments the call gave a value, in the order they are declared.
fn named(arguments: &Arguments) -> Vec<&'static str> {
    let block = |value: &Option<Value>| value.as_ref().is_some_and(|value| !value.is_null());
    let asked: [(&'static str, bool); 29] = [
        ("text", arguments.text.is_some()),
        ("font", arguments.font.is_some()),
        ("weight", arguments.weight.is_some()),
        ("italic", arguments.italic.is_some()),
        ("size", arguments.size.is_some()),
        ("color", arguments.color.is_some()),
        ("align", arguments.align.is_some()),
        ("line_height", arguments.line_height.is_some()),
        ("max_width", arguments.max_width.is_some()),
        ("icon", arguments.icon.is_some()),
        ("geometry", arguments.geometry.is_some()),
        ("width", arguments.width.is_some()),
        ("height", arguments.height.is_some()),
        ("radius", arguments.radius.is_some()),
        ("fill", arguments.fill.is_some()),
        ("stroke", arguments.stroke.is_some()),
        ("stroke_width", arguments.stroke_width.is_some()),
        ("from", arguments.from.is_some()),
        ("to", arguments.to.is_some()),
        ("curve", arguments.curve.is_some()),
        ("heads", arguments.heads.is_some()),
        ("dash", arguments.dash.is_some()),
        ("reveal", block(&arguments.reveal)),
        ("number", block(&arguments.number)),
        ("prompt", arguments.prompt.is_some()),
        ("recipe", arguments.recipe.is_some()),
        ("video", block(&arguments.video)),
        ("image", block(&arguments.image)),
        ("speech", block(&arguments.speech)),
    ];
    asked
        .into_iter()
        .filter_map(|(field, asked)| asked.then_some(field))
        .collect()
}

/// What a `kind` is called *in the document*.
///
/// Read out of the format rather than off `Debug`, which would answer
/// `GeneratedVideo` to somebody looking at `"kind": "generated_video"` in the
/// file they are being told about.
pub(super) fn kind_name(kind: AssetKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{kind:?}"))
}
