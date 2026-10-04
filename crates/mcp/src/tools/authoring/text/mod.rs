//! A caption, a title, a lower third: the one asset an agent writes most.

mod motion;

pub(super) use motion::{number_property, reveal_property};

use schemars::JsonSchema;
use scorsese_core::{Inline, TextStyle, authoring};
use serde::Deserialize;
use serde_json::Value;

use super::{
    ALIGN, Align, COLOR, FONT, ITALIC, LINE_HEIGHT, MAX_WIDTH, SIZE, STROKE, STROKE_WIDTH, WEIGHT,
    color, id_described, maybe, refused, save, weight,
};
use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Add a `text` asset.
pub(crate) struct TextNew;

/// What the caption says, when it is missing.
const WHAT: &str = "what the caption says";

/// What `text_new` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// What it says. Newlines are kept, and anything longer than `max_width`
    /// wraps on its own. An emoji renders, in colour and at the size of the
    /// line it sits in — nothing selects it and nothing has to.
    text: String,
    #[schemars(description = FONT)]
    font: Option<String>,
    #[schemars(description = WEIGHT)]
    weight: Option<u32>,
    #[schemars(description = ITALIC)]
    italic: Option<bool>,
    #[schemars(description = SIZE)]
    size: Option<f64>,
    #[schemars(description = COLOR)]
    color: Option<String>,
    #[serde(default)]
    #[schemars(with = "Align", description = ALIGN)]
    align: Option<Align>,
    #[schemars(description = LINE_HEIGHT)]
    line_height: Option<f64>,
    #[schemars(description = MAX_WIDTH)]
    max_width: Option<f64>,
    #[schemars(description = STROKE)]
    stroke: Option<String>,
    #[schemars(description = STROKE_WIDTH)]
    stroke_width: Option<f64>,
    // Described by its schema, which spells out the block's own fields.
    #[serde(default)]
    #[schemars(schema_with = "motion::reveal_schema")]
    reveal: Option<Value>,
    // The same, for the counter.
    #[serde(default)]
    #[schemars(schema_with = "motion::number_schema")]
    number: Option<Value>,
    #[schemars(description = id_described("its opening words"))]
    asset: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("text", WHAT)];
}

impl Arguments {
    /// The look a new caption is set in, or `None` when nothing about it was
    /// said — which leaves the style out of the document rather than writing
    /// the defaults into it.
    fn style(&self) -> Result<Option<TextStyle>, String> {
        let mut style = TextStyle::default();
        let mut said = false;
        if let Some(font) = maybe(self.font.as_deref()) {
            style.font = font.into();
            said = true;
        }
        if let Some(weight) = weight(self.weight)? {
            style.weight = Some(weight);
            said = true;
        }
        if let Some(italic) = self.italic {
            style.italic = italic;
            said = true;
        }
        for (value, field) in [
            (self.size, &mut style.size),
            (self.line_height, &mut style.line_height),
            (self.max_width, &mut style.max_width),
            (self.stroke_width, &mut style.stroke_width),
        ] {
            if let Some(value) = value {
                *field = value;
                said = true;
            }
        }
        if let Some(color) = color(self.color.as_deref(), "color")? {
            style.color = color;
            said = true;
        }
        if let Some(align) = self.align {
            style.align = align.into();
            said = true;
        }
        if let Some(stroke) = color(self.stroke.as_deref(), "stroke")? {
            style.stroke = Some(stroke);
            said = true;
        }
        Ok(said.then_some(style))
    }
}

impl Tool for TextNew {
    fn name(&self) -> &'static str {
        "text_new"
    }

    fn description(&self) -> &'static str {
        "Add a text asset — a caption, a title, a lower third: what it says, and \
         the look it is set in. This is the single most common thing there is to \
         author in a cut, and the alternative is sending the whole project.json \
         back to change one line. The string lives in the document, so there is \
         no file to import, hash or probe, and nothing to generate. Sizes are \
         fractions of the frame rather than pixels, so one number reads the same \
         at every render resolution. Say nothing about the look and it is a \
         white, centred, sans title; a `stroke` adds a rim outside the \
         letterforms, which is what keeps words legible over footage. A \
         `reveal` block and a `reveal` keyframe track make it arrive word by word \
         (or by character, or line); a `number` block writes a figure at `{n}` \
         that counts with a `number` track. \
         Validated before it is written — a document \
         that would not load is refused with the reason and the project is left \
         exactly as it was. Then place_clip puts it on a video track, and it has \
         no length of its own, so that call needs a duration."
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
        // Kept as written, newlines and all — only a caption with nothing in
        // it is refused, the way a missing one is.
        if arguments.text.trim().is_empty() {
            return Err(format!("`text` is required: {WHAT}"));
        }
        let style = motion::apply(
            arguments.reveal.as_ref(),
            arguments.number.as_ref(),
            arguments.style()?,
        )?;
        let content = Inline::Text {
            text: arguments.text.clone(),
            style,
        };
        let id = authoring::add_asset(
            &mut project,
            maybe(arguments.asset.as_deref()).as_deref(),
            content,
        )
        .map_err(refused)?;
        save(&project, dir)?;
        Ok(format!(
            "`{id}` — a text asset. place_clip puts it on a video track, with a \
             duration: a title has no length of its own."
        )
        .into())
    }
}
