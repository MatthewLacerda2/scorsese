//! Changing one number in a recipe, rather than the whole document.

use scorsese_providers::synth;
use serde_json::Value;

use super::recipes::{read, recipe_path, recipe_property, text};
use crate::tools::{Costs, Reply, Tool, project_property};

/// Set one field of a recipe.
pub(crate) struct Set;

impl Tool for Set {
    fn name(&self) -> &'static str {
        "synth_set"
    }

    fn description(&self) -> &'static str {
        "Change one number in a recipe and leave the rest of the document alone: \
         a track's gain or pan, or the recipe's own bpm, seed, swing, duration \
         or velocity. Reach for this while **tuning** — chasing a level over \
         several bakes — where synth_write would re-send every note in the piece \
         to move one float. Refused whole, changing nothing, if the field or the \
         track named is not there. The result is an ordinary recipe, and its \
         asset goes stale by the same arithmetic a full write does."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "recipe": recipe_property(),
                "field": {
                    "type": "string",
                    "enum": synth::FIELDS,
                    "description": "Which number to change. On a song: `bpm`, `seed`, \
                                    `swing`, `gain` or `pan` — and the last two are a \
                                    track's, so they need `track`. On a patch: \
                                    `duration`, `velocity`, `seed`. Anything else, \
                                    including a note or an arrangement entry, is a \
                                    synth_write."
                },
                "track": {
                    "type": "string",
                    "description": "The name of the track to change — the same name the \
                                    song's notes use. Only for `gain` and `pan`."
                },
                "value": {
                    "type": "number",
                    "description": "What the field becomes. `seed` is a whole number \
                                    and not a negative one; the rest are decimals."
                }
            },
            "required": ["project", "recipe", "field", "value"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let (_, file, relative) = recipe_path(arguments)?;
        let field = text(arguments, "field")?;
        let value = arguments
            .get("value")
            .and_then(Value::as_f64)
            .ok_or_else(|| "`value` is required: the number to set".to_owned())?;
        let track = arguments
            .get("track")
            .and_then(Value::as_str)
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
