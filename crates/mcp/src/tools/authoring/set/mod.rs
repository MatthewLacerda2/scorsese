//! `asset_set`: the one tool that writes an asset carried in the document —
//! making it when the id names nothing yet, changing the fields named when it
//! does (#780).
//!
//! It was five tools and a sixth: `text_new`, `color_new`, `shape_new` and
//! `icon_new` made what `asset_set` changed, field for field, and `rebrief`
//! changed the one field a generated asset has. Their schemas described the
//! same properties twice, they were a quarter of every `tools/list` a model
//! call carries, and the record #779 read showed models choosing a
//! whole-document `project_write` over all of them. `sequence` already worked
//! this way — an id nothing answers to is made — so this is that precedent,
//! extended to every kind whose content the document carries.
//!
//! **What a kind requires is said in prose, not in the schema.** The `*_new`
//! tools could mark `color` required for a colour card because each schema
//! was one kind's; one schema for every kind cannot, so the requirement moved
//! into each argument's description and into the refusal, which names the
//! missing argument the same way [`args::parse`] does. A field a kind has no
//! use for is refused by name ([`fields`]), never ignored.
//!
//! **`kind` on an asset that exists is accepted when it matches and refused
//! when it does not.** An agent re-sending a create should not fail on the
//! second try, and a call that says `shape` about a caption has misunderstood
//! which asset it is writing — guessing which half was meant would change the
//! asset nobody checked.
//!
//! **`sequence` stays its own tool.** Its fields — stills, hold, loop — share
//! nothing with these, so folding it in would add three arguments here for
//! one kind and save one name; it already creates-or-changes on its own.

mod arrow;
mod brief;
mod change;
mod fields;
mod make;
mod motion;
mod requests;
mod shape;
mod text;

use schemars::JsonSchema;
use scorsese_core::{AssetId, AssetKind};
use serde::Deserialize;
use serde_json::Value;

use super::fill::{self, Paint};
use super::{
    ALIGN, Align, FILL, FONT, HEIGHT, ITALIC, LINE_HEIGHT, MAX_WIDTH, RADIUS, SIZE, STROKE,
    STROKE_WIDTH, WEIGHT, WIDTH, maybe,
};
use crate::tools::args::{self, ProjectDir};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Make or change a text, color, shape or icon asset, or a generated one's
/// brief.
pub(crate) struct AssetSet;

/// The kinds this tool can make.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Text,
    Color,
    Shape,
    Icon,
    GeneratedVideo,
    GeneratedImage,
    GeneratedAudio,
}

impl From<Kind> for AssetKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Text => Self::Text,
            Kind::Color => Self::Color,
            Kind::Shape => Self::Shape,
            Kind::Icon => Self::Icon,
            Kind::GeneratedVideo => Self::GeneratedVideo,
            Kind::GeneratedImage => Self::GeneratedImage,
            Kind::GeneratedAudio => Self::GeneratedAudio,
        }
    }
}

/// Which kinds take an argument, ahead of what it is.
fn on(kinds: &str, what: &str) -> String {
    format!("({kinds}) {what}")
}

/// What `asset_set` takes: the asset, its kind when making one, and any of
/// the fields its kind has.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The asset's id. One that exists is changed; one nothing answers to is
    /// made, which takes `kind`. Left out with a `kind`, an id is derived from
    /// the content (a caption's or a prompt's opening words, a shape's
    /// outline, an icon's symbol) and suffixed until it is free; the reply
    /// says which.
    asset: Option<String>,
    /// What to make: `text`, `color`, `shape`, `icon`, or a free sketch of a
    /// `generated_video`, `generated_image` or `generated_audio`. Required to
    /// make one; on an existing asset it may be left out, and one that is not
    /// the asset's kind is refused. A web page is page_write's, a synth_audio
    /// asset synth_new's, an image sequence sequence's.
    kind: Option<Kind>,
    /// (text) What it says — required to make one, and replaced whole when
    /// changed. Newlines are kept and anything longer than `max_width` wraps;
    /// an emoji renders, in colour, at the size of its line.
    text: Option<String>,
    #[schemars(description = on("text", FONT))]
    font: Option<String>,
    #[schemars(description = on("text", WEIGHT))]
    weight: Option<u32>,
    #[schemars(description = on("text", ITALIC))]
    italic: Option<bool>,
    #[schemars(description = on("text, icon", SIZE))]
    size: Option<f64>,
    #[schemars(description = fill::described(
        "(text, color, icon) The colour, as `#rrggbb` (or `#rrggbbaa` for a scrim the \
         shot underneath shows through). Required to make a color or an icon asset, and \
         with no default on purpose: a card or a symbol nobody chose the colour of still \
         renders, wrongly, with nothing to say so. A text or icon asset is one colour; \
         only a color asset takes the gradient form — a radial one lighter behind the \
         subject is the usual backdrop."
    ))]
    color: Option<Paint>,
    #[schemars(description = on("text", ALIGN))]
    align: Option<Align>,
    #[schemars(description = on("text", LINE_HEIGHT))]
    line_height: Option<f64>,
    #[schemars(description = on("text", MAX_WIDTH))]
    max_width: Option<f64>,
    /// (icon) Which symbol, by the catalogue's own name — lowercase and
    /// hyphenated, `clapperboard` or `circle-play`; the `icons` tool finds one
    /// from a word. Required to make one, with `size` and `color`. A name the
    /// catalogue lacks is refused by project_check and the render, not here.
    icon: Option<String>,
    /// (shape, when making it) Which outline. `rectangle` and `ellipse` need a
    /// `width` and a `height`; `arrow` needs `from` and `to` and has no size of
    /// its own. Required to make a shape.
    geometry: Option<shape::Outline>,
    #[schemars(description = on("shape", WIDTH))]
    width: Option<f64>,
    #[schemars(description = on("shape", HEIGHT))]
    height: Option<f64>,
    #[schemars(description = on("shape", RADIUS))]
    radius: Option<f64>,
    #[schemars(description = on("shape", FILL))]
    fill: Option<Paint>,
    #[schemars(description = on("shape, text", STROKE))]
    stroke: Option<String>,
    #[schemars(description = on("shape, text, icon", STROKE_WIDTH))]
    stroke_width: Option<f64>,
    #[schemars(description = arrow::endpoint_described("starts"))]
    from: Option<arrow::End>,
    // Described by its schema, which points at `from` rather than repeat it.
    #[serde(default)]
    #[schemars(schema_with = "arrow::to_schema")]
    to: Option<arrow::End>,
    /// (shape: an arrow, when making it) How it gets from one end to the
    /// other. `straight` is the default; `s` bows it so it leaves and arrives
    /// along the same axis — a connector between two boxes side by side.
    curve: Option<arrow::Line>,
    /// (shape: an arrow, when making it) Which ends carry a head. `end` is the
    /// default and points at `to`; `none` is a plain line; `both` says the two
    /// are connected, without a direction.
    heads: Option<arrow::Tips>,
    // Described by its schema, which also bounds each length.
    #[serde(default)]
    #[schemars(schema_with = "shape::dash_schema")]
    dash: Option<Vec<f64>>,
    // Described by its schema: the block's own fields, and what `false` does.
    #[serde(default)]
    #[schemars(schema_with = "change::reveal_change")]
    reveal: Option<Value>,
    // The same, for the counter.
    #[serde(default)]
    #[schemars(schema_with = "change::number_change")]
    number: Option<Value>,
    /// (generated_video, generated_image, generated_audio) The sentence a
    /// provider will be paid to read: what the shot is of, what the still
    /// shows, the words spoken. Required to make one; replaces the old one
    /// whole. docs/prompts.md has what certain words do.
    prompt: Option<String>,
    /// (synth_audio) Project-relative path of the recipe the asset is baked
    /// from, by convention under recipes/; the file has to exist. This
    /// repoints the asset — what is inside a recipe is synth_write's or
    /// synth_set's.
    recipe: Option<String>,
    // Described by their schemas: each brief block's fields and defaults.
    #[serde(default)]
    #[schemars(schema_with = "requests::video_schema")]
    video: Option<Value>,
    #[serde(default)]
    #[schemars(schema_with = "requests::image_schema")]
    image: Option<Value>,
    #[serde(default)]
    #[schemars(schema_with = "requests::speech_schema")]
    speech: Option<Value>,
}

impl args::Arguments for Arguments {}

impl Tool for AssetSet {
    fn name(&self) -> &'static str {
        "asset_set"
    }

    fn description(&self) -> &'static str {
        "Make or change a caption, title, color card, shape or icon asset, or the \
         free sketch and brief of a generated shot, still or spoken line. A text is a \
         caption, a title or a lower third; a shape a rectangle, an ellipse or \
         an arrow. **To make one, give its `kind`**; then place_clip puts it on \
         a track, with a duration. **To change one, name its \
         `asset`**: every argument you leave out stays exactly as it is, so \
         setting a size does not reset a font chosen two turns ago, and the \
         reply says what each field was and is now. Each argument says which \
         kinds take it, and one the kind has no use for is refused by name. \
         Sizes are fractions of the frame, so one document reads the same at \
         every resolution. A text whose look nobody set is a white, centred, \
         sans title; a shape with neither a fill nor a border draws nothing and \
         is refused, because it would look exactly like one that failed. A \
         generated asset is made in state sketch and renders as a slug card, so \
         a cut previews for $0 — nothing is generated or spent here; generate \
         realises it. A new brief (`prompt`, `recipe`, a field of `video`, \
         `image` or `speech`) on one already generated marks it stale in the \
         same write, so the next generate redoes it; a sketch stays sketch. \
         Nothing is written unless the whole document still loads."
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
        let named = maybe(arguments.asset.as_deref());
        let existing = named
            .as_deref()
            .and_then(|id| project.asset(&AssetId::new(id)))
            .map(|asset| asset.kind);
        let asked = arguments.kind.map(AssetKind::from);
        let said = match (existing, asked, named) {
            (Some(kind), Some(asked), Some(id)) if kind != asked => {
                return Err(format!(
                    "`{id}` is a {} asset, not a {} one — nothing was written. Leave \
                     `kind` out to change it, or name a new id to make one",
                    fields::kind_name(kind),
                    fields::kind_name(asked)
                ));
            }
            (Some(kind), _, Some(id)) => {
                fields::check(kind, false, &arguments)?;
                change::change(&mut project, dir, &AssetId::new(id), kind, &arguments)?
            }
            (_, Some(kind), named) => {
                fields::check(kind, true, &arguments)?;
                make::make(&mut project, dir, named.as_deref(), kind, &arguments)?
            }
            (_, None, Some(id)) => {
                return Err(format!(
                    "there is no asset `{id}` — to make one, say what `kind` it is"
                ));
            }
            (_, None, None) => {
                return Err("`asset` is required: the id of the asset to change — or \
                            give a `kind` to make a new one"
                    .to_owned());
            }
        };
        Ok(said.into())
    }
}
