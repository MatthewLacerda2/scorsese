//! Changing one field of an asset that carries its content in the document.

use scorsese_core::{AssetId, BlockChange, Edit, authoring};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::fill::{self, fill};
use super::text::{number_property, reveal_property};
use super::{align, color, maybe, number, properties, refused, save, weight, words};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir};

/// Change a field on a `text`, `color`, `shape` or `icon` asset.
pub(crate) struct AssetSet;

impl Tool for AssetSet {
    fn name(&self) -> &'static str {
        "asset_set"
    }

    fn description(&self) -> &'static str {
        "Change a field on an asset that carries its content in the document — a \
         text, color, shape or icon asset: its wording, its size, its colour. \
         This is the loop the whole family exists for: reword a caption, drop its \
         size by a hundredth, look at a still, do it again. **Every argument you \
         leave out is left exactly as it is**, so setting a size does not reset a \
         font somebody chose two turns ago — which is what sending a whole style \
         block back would do, and would say nothing about. Each field belongs to \
         certain kinds and a field the asset's kind has no use for is refused by \
         name rather than quietly ignored. The reply says what each field was as \
         well as what it is now. Nothing is written unless the whole document \
         still loads. A text's `reveal` and `number` blocks merge the same way one \
         level down, and `false` takes either away. What a generated asset is made \
         from is rebrief, and what a file-backed one is lives in the file."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        let mut properties = properties(&[
            "font",
            "weight",
            "italic",
            "size",
            "color",
            "align",
            "line_height",
            "max_width",
            "fill",
            "stroke",
            "stroke_width",
            "width",
            "height",
            "radius",
        ]);
        properties.insert(
            "color".to_owned(),
            fill::property(
                "The colour, as `#rrggbb` (or `#rrggbbaa`). A text or icon asset is one \
                 colour; only a color asset takes the gradient form.",
            ),
        );
        properties.insert(
            "asset".to_owned(),
            serde_json::json!({
                "type": "string",
                "description": "Id of the asset to change. It must be a text, color, \
                                shape or icon asset — the kinds whose content is in the \
                                document. project_assets lists them."
            }),
        );
        properties.insert(
            "text".to_owned(),
            serde_json::json!({
                "type": "string",
                "description": "What a `text` asset says. Replaces the whole string, \
                                which is what rewording a caption means."
            }),
        );
        properties.insert(
            "icon".to_owned(),
            serde_json::json!({
                "type": "string",
                "description": "Which symbol an `icon` asset draws — its `name` field, \
                                not the asset's id. `icons` finds one from a word."
            }),
        );
        properties.insert(
            "reveal".to_owned(),
            mergeable(
                reveal_property(),
                "A text asset's reveal — how it arrives a piece at a time when a `reveal` \
                 keyframe track on its clip goes 0 to 1: by `char` (the typewriter), \
                 `word` or `line`. An object sets the fields it names and keeps the \
                 rest (on a caption without one: by word, rise 0.2, stagger 0.5), so \
                 `{\"unit\": \"char\"}` alone turns word by word into letter by letter; \
                 `false` removes it, and the text simply shows.",
            ),
        );
        properties.insert(
            "number".to_owned(),
            mergeable(
                number_property(),
                "The figure a text asset writes where it says `{n}`, and counts with a \
                 `number` keyframe track. An object sets the fields it names and keeps \
                 the rest (on a caption without one: value 0, no decimals, `en`, \
                 grouped), so `{\"value\": 144}` alone changes what it counts to; \
                 `false` removes it, after which a `{n}` left in the text is just \
                 those three characters. The text must contain `{n}` — reword it in \
                 the same call if it does not.",
            ),
        );
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": ["project", "asset"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let id = AssetId::new(words(arguments, "asset", "the id of the asset to change")?);
        let edit = Edit {
            text: maybe(arguments, "text"),
            font: maybe(arguments, "font"),
            weight: weight(arguments)?,
            italic: arguments.get("italic").and_then(Value::as_bool),
            size: number(arguments, "size")?,
            color: fill(arguments, "color")?,
            align: align(arguments)?,
            line_height: number(arguments, "line_height")?,
            max_width: number(arguments, "max_width")?,
            icon: maybe(arguments, "icon"),
            fill: fill(arguments, "fill")?,
            stroke: color(arguments, "stroke")?,
            stroke_width: number(arguments, "stroke_width")?,
            width: number(arguments, "width")?,
            height: number(arguments, "height")?,
            radius: number(arguments, "radius")?,
            reveal: block(arguments, "reveal")?,
            number: block(arguments, "number")?,
        };
        let changed = authoring::set_asset(&mut project, &id, &edit).map_err(refused)?;
        save(&project, &dir)?;
        Ok(format!("`{id}`: {}. Nothing else changed.", changed.join(", ")).into())
    }
}

/// A block's schema as `asset_set` takes it: an object that merges, or `false`.
fn mergeable(mut schema: Value, description: &str) -> Value {
    schema["type"] = serde_json::json!(["object", "boolean"]);
    schema["description"] = Value::from(description);
    schema["additionalProperties"] = Value::Bool(false);
    schema
}

/// `false` for none; an object for the fields to change, read into the
/// document's own types so a word that is not a unit is refused as a
/// hand-written document's would be. `null` counts as not given.
fn block<T: DeserializeOwned>(
    arguments: &Value,
    key: &str,
) -> Result<Option<BlockChange<T>>, String> {
    match arguments.get(key) {
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
