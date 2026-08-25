//! Changing one field of an asset that carries its content in the document.

use scorsese_core::{AssetId, Edit, authoring};
use serde_json::Value;

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
         still loads. What a generated asset is made from is rebrief, and what a \
         file-backed one is lives in the file."
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
            color: color(arguments, "color")?,
            align: align(arguments)?,
            line_height: number(arguments, "line_height")?,
            max_width: number(arguments, "max_width")?,
            icon: maybe(arguments, "icon"),
            fill: color(arguments, "fill")?,
            stroke: color(arguments, "stroke")?,
            stroke_width: number(arguments, "stroke_width")?,
            width: number(arguments, "width")?,
            height: number(arguments, "height")?,
            radius: number(arguments, "radius")?,
        };
        let changed = authoring::set_asset(&mut project, &id, &edit).map_err(refused)?;
        save(&project, &dir)?;
        Ok(format!("`{id}`: {}. Nothing else changed.", changed.join(", ")).into())
    }
}
