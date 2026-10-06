//! Changing the fields named on an asset that exists, and leaving the rest
//! alone.

use schemars::{Schema, SchemaGenerator};
use scorsese_core::{AssetId, AssetKind, BlockChange, Edit, Project, authoring};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::super::fill::fill;
use super::super::{color, maybe, refused, save, weight};
use super::motion::{number_property, reveal_property};
use super::{Arguments, brief};

/// Changes `id`, a `kind` asset, in exactly the fields the arguments name,
/// writes the document, and says what each field became.
pub(super) fn change(
    project: &mut Project,
    dir: &std::path::Path,
    id: &AssetId,
    kind: AssetKind,
    arguments: &Arguments,
) -> Result<String, String> {
    if kind.is_generated() {
        return brief::change(project, dir, id, arguments);
    }
    let edit = Edit {
        text: maybe(arguments.text.as_deref()),
        font: maybe(arguments.font.as_deref()),
        weight: weight(arguments.weight)?,
        italic: arguments.italic,
        size: arguments.size,
        color: fill(arguments.color.as_ref(), "color")?,
        align: arguments.align.map(Into::into),
        line_height: arguments.line_height,
        max_width: arguments.max_width,
        icon: maybe(arguments.icon.as_deref()),
        fill: fill(arguments.fill.as_ref(), "fill")?,
        stroke: color(arguments.stroke.as_deref(), "stroke")?,
        stroke_width: arguments.stroke_width,
        width: arguments.width,
        height: arguments.height,
        radius: arguments.radius,
        reveal: block(arguments.reveal.as_ref(), "reveal")?,
        number: block(arguments.number.as_ref(), "number")?,
    };
    let changed = authoring::set_asset(project, id, &edit).map_err(refused)?;
    save(project, dir)?;
    Ok(format!(
        "`{id}`: {}. Nothing else changed.",
        changed.join(", ")
    ))
}

/// The `reveal` block as `asset_set` takes it.
pub(super) fn reveal_change(_: &mut SchemaGenerator) -> Schema {
    mergeable(
        reveal_property(),
        "(text) Its reveal — how it arrives a piece at a time when a `reveal` \
         keyframe track on its clip goes 0 to 1: by `char` (the typewriter), \
         `word` or `line`. An object sets the fields it names and keeps the \
         rest (on a caption without one: by word, rise 0.2, stagger 0.5), so \
         `{\"unit\": \"char\"}` alone turns word by word into letter by letter; \
         `false` removes it, and the text simply shows.",
    )
}

/// The `number` block as `asset_set` takes it.
pub(super) fn number_change(_: &mut SchemaGenerator) -> Schema {
    mergeable(
        number_property(),
        "(text) The figure it writes where it says `{n}`, and counts with a \
         `number` keyframe track. An object sets the fields it names and keeps \
         the rest (on a caption without one: value 0, no decimals, `en`, \
         grouped), so `{\"value\": 144}` alone changes what it counts to; \
         `false` removes it, after which a `{n}` left in the text is just \
         those three characters. The text must contain `{n}` — reword it in \
         the same call if it does not.",
    )
}

/// A block's schema as `asset_set` takes it: an object that merges, or `false`.
fn mergeable(mut schema: Value, description: &str) -> Schema {
    schema["type"] = serde_json::json!(["object", "boolean"]);
    schema["description"] = Value::from(description);
    schema["additionalProperties"] = Value::Bool(false);
    Schema::try_from(schema).expect("a block's schema is an object")
}

/// `false` for none; an object for the fields to change, read into the
/// document's own types so a word that is not a unit is refused as a
/// hand-written document's would be. `null` counts as not given.
fn block<T: DeserializeOwned>(
    given: Option<&Value>,
    key: &str,
) -> Result<Option<BlockChange<T>>, String> {
    match given {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(false)) => Ok(Some(BlockChange::Remove)),
        Some(fields @ Value::Object(_)) => serde_json::from_value(fields.clone())
            .map(|fields| Some(BlockChange::Merge(fields)))
            .map_err(|error| format!("`{key}`: {error}")),
        Some(other) => Err(format!(
            "`{key}` is an object of fields, or `false` to remove it, not {other}"
        )),
    }
}
