//! Changing one number in a recipe, rather than the whole document.

use schemars::JsonSchema;
use scorsese_providers::synth;
use serde::Deserialize;
use serde_json::Value;

use super::recipes::{RECIPE, read, recipe_path};
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::{Costs, Reply, Tool};

/// Set one field of a recipe.
pub(crate) struct Set;

/// What `synth_set` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    #[schemars(description = RECIPE)]
    recipe: Name,
    /// Which number to change. On a song: `bpm`, `seed`, `swing`, `gain`,
    /// `pan` or `send` — and the last three are a track's, so they need
    /// `track`. `send` (0 to 1, default 1) is how much of the track the song's
    /// reverb and delay hear. On a patch: `duration`, `velocity`, `seed`.
    /// Anything else, including a note or an arrangement entry, is a
    /// synth_write.
    // A string checked by the synthesiser rather than an enum here, so that a
    // field it does not know is refused in its words, with nothing written.
    #[schemars(extend("enum" = synth::FIELDS))]
    field: Name,
    /// What the field becomes. `seed` is a whole number and not a negative
    /// one; the rest are decimals.
    value: f64,
    /// The name of the track to change — the same name the song's notes use.
    /// Only for `gain`, `pan` and `send`.
    track: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("value", "the number to set")];
}

impl Tool for Set {
    fn name(&self) -> &'static str {
        "synth_set"
    }

    fn description(&self) -> &'static str {
        "Change one number in a recipe and leave the rest of the document alone: \
         a track's gain, pan or send, or the recipe's own bpm, seed, swing, duration \
         or velocity. Reach for this while **tuning** — chasing a level over \
         several bakes — where synth_write would re-send every note in the piece \
         to move one float. Refused whole, changing nothing, if the field or the \
         track named is not there. The result is an ordinary recipe, and its \
         asset goes stale by the same arithmetic a full write does. One thing it \
         cannot reach: a track whose `gain` or `pan` is under an automation curve \
         plays the curve and not the written number, so setting it there changes \
         a value nothing reads — move the curve's points with synth_write."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let (file, relative) = recipe_path(arguments.project.dir(), &arguments.recipe)?;
        let field = arguments.field.as_str();
        let value = arguments.value;
        // Blank is no track, as it always was; the name itself is passed as
        // written, since it has to match the song's own.
        let track = arguments
            .track
            .as_deref()
            .filter(|name| !name.trim().is_empty());

        let json = read(&file)?;
        let setting = synth::Setting {
            field,
            track,
            value,
        };
        // Every refusal lands here, before anything is written, which is what
        // makes "changed nothing" true rather than merely likely.
        let change = synth::set(&json, &setting)
            .map_err(|problem| format!("refused, nothing written — {relative}: {problem}"))?;
        scorsese_core::write::atomically(&file, &change.document)
            .map_err(|error| format!("writing {relative}: {error}"))?;

        let target = match track {
            Some(name) => format!("`{field}` on track `{name}`"),
            None => format!("`{field}`"),
        };
        Ok(format!(
            "{relative}: {target} {} → {value} — a {} recipe, and no other value \
             changed. Its asset is stale now; synth_bake redoes it.",
            change.was, change.kind
        )
        .into())
    }
}
