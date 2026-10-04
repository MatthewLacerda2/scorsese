//! Changing one field of an asset that carries its content in the document.

use schemars::{JsonSchema, Schema, SchemaGenerator};
use scorsese_core::{AssetId, BlockChange, Edit, authoring};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::fill::{self, Paint, fill};
use super::text::{number_property, reveal_property};
use super::{
    ALIGN, Align, FILL, FONT, HEIGHT, ITALIC, LINE_HEIGHT, MAX_WIDTH, RADIUS, SIZE, STROKE,
    STROKE_WIDTH, WEIGHT, WIDTH, color, maybe, refused, save, weight,
};
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Change a field on a `text`, `color`, `shape` or `icon` asset.
pub(crate) struct AssetSet;

/// What `asset_set` takes: the asset, and any of the fields its kind has.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the asset to change. It must be a text, color, shape or icon
    /// asset — the kinds whose content is in the document. project_assets
    /// lists them.
    asset: Name,
    /// What a `text` asset says. Replaces the whole string, which is what
    /// rewording a caption means.
    text: Option<String>,
    #[schemars(description = FONT)]
    font: Option<String>,
    #[schemars(description = WEIGHT)]
    weight: Option<u32>,
    #[schemars(description = ITALIC)]
    italic: Option<bool>,
    #[schemars(description = SIZE)]
    size: Option<f64>,
    #[schemars(description = fill::described(
        "The colour, as `#rrggbb` (or `#rrggbbaa`). A text or icon asset is one colour; \
         only a color asset takes the gradient form."
    ))]
    color: Option<Paint>,
    #[serde(default)]
    #[schemars(with = "Align", description = ALIGN)]
    align: Option<Align>,
    #[schemars(description = LINE_HEIGHT)]
    line_height: Option<f64>,
    #[schemars(description = MAX_WIDTH)]
    max_width: Option<f64>,
    /// Which symbol an `icon` asset draws — its `name` field, not the asset's
    /// id. `icons` finds one from a word.
    icon: Option<String>,
    #[schemars(description = fill::described(FILL))]
    fill: Option<Paint>,
    #[schemars(description = STROKE)]
    stroke: Option<String>,
    #[schemars(description = STROKE_WIDTH)]
    stroke_width: Option<f64>,
    #[schemars(description = WIDTH)]
    width: Option<f64>,
    #[schemars(description = HEIGHT)]
    height: Option<f64>,
    #[schemars(description = RADIUS)]
    radius: Option<f64>,
    // Described by its schema: the block's own fields, and what `false` does.
    #[serde(default)]
    #[schemars(schema_with = "reveal_change")]
    reveal: Option<Value>,
    // The same, for the counter.
    #[serde(default)]
    #[schemars(schema_with = "number_change")]
    number: Option<Value>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("asset", "the id of the asset to change")];
}

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
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let id = AssetId::new(arguments.asset.as_str());
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
        let changed = authoring::set_asset(&mut project, &id, &edit).map_err(refused)?;
        save(&project, dir)?;
        Ok(format!("`{id}`: {}. Nothing else changed.", changed.join(", ")).into())
    }
}

/// The `reveal` block as `asset_set` takes it.
fn reveal_change(_: &mut SchemaGenerator) -> Schema {
    mergeable(
        reveal_property(),
        "A text asset's reveal — how it arrives a piece at a time when a `reveal` \
         keyframe track on its clip goes 0 to 1: by `char` (the typewriter), \
         `word` or `line`. An object sets the fields it names and keeps the \
         rest (on a caption without one: by word, rise 0.2, stagger 0.5), so \
         `{\"unit\": \"char\"}` alone turns word by word into letter by letter; \
         `false` removes it, and the text simply shows.",
    )
}

/// The `number` block as `asset_set` takes it.
fn number_change(_: &mut SchemaGenerator) -> Schema {
    mergeable(
        number_property(),
        "The figure a text asset writes where it says `{n}`, and counts with a \
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
