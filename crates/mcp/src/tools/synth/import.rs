//! A MIDI file in as a song recipe: `synth_import`.
//!
//! The front door for a piece that already exists as notes somewhere else — a
//! DAW export, a keyboard take, a transcription. What it writes is an ordinary
//! recipe, so everything after it is the usual loop: read it, change the
//! sounds, bake.

use scorsese_providers::synth;
use serde_json::Value;

use super::text;
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property, under};

/// Read a `.mid` into a recipe.
pub(crate) struct Import;

impl Tool for Import {
    fn name(&self) -> &'static str {
        "synth_import"
    }

    fn description(&self) -> &'static str {
        "Read a Standard MIDI File into a song recipe in recipes/ and add the \
         synth_audio asset that points at it, the way synth_new does. The \
         file's structure comes across as written: one song track per MIDI \
         track and channel, channel 10 as a drum part whose notes stay key \
         numbers, the tempo map, the key signature, every note with its \
         velocity, cut into patterns of eight bars named for the bars they \
         hold. Nothing is interpreted — no hands split, no repeats found — and \
         every track plays a plain placeholder patch, so choosing the sounds is \
         the first synth_write. The reply lists what the song cannot hold (the \
         sustain pedal, pitch bends, program numbers) rather than dropping it \
         silently. Costs nothing."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "path": {
                    "type": "string",
                    "description": "The .mid file to read — relative to the project \
                                    directory, or absolute. It is read, not copied: \
                                    the recipe it becomes is what the project keeps."
                },
                "name": {
                    "type": "string",
                    "description": "What to call the asset and its recipe. Defaults \
                                    to the file's name without .mid, suffixed if \
                                    that is taken."
                }
            },
            "required": ["project", "path"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let file = under(&dir, arguments, "path")?.ok_or("`path` is required: the .mid file")?;
        let name = match arguments.get("name") {
            Some(_) => Some(
                text(arguments, "name")
                    .map_err(|_| "`name`, when given, is a non-empty string".to_owned())?,
            ),
            None => None,
        };
        let mut project = load(&dir)?;
        let imported = synth::import_midi(&mut project, &dir, &file, name)
            .map_err(|error| format!("{error}"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        let id = &imported.id;
        let recipe = project
            .asset(id)
            .and_then(|asset| asset.recipe.as_ref())
            .map(ToString::to_string)
            .unwrap_or_default();
        let mut lines = vec![format!("{id} — synth_audio, sketch"), recipe];
        lines.extend(imported.lines());
        lines.push(
            "Choose the sounds with synth_read and synth_write, then synth_bake to hear it."
                .to_owned(),
        );
        Ok(lines.join("\n").into())
    }
}
