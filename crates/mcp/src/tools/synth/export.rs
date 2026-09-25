//! A song recipe out as a MIDI file: `synth_export`.
//!
//! `synth_import` the other way round, and the door out to every other tool a
//! musician owns: a `.mid` opens in any DAW, which is where a user checks or
//! finishes zimmer's work with the tools they already know.

use scorsese_core::AssetId;
use scorsese_providers::synth::{self, Drum};
use serde_json::Value;

use super::text;
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property, under};

/// Write a song recipe as a `.mid`.
pub(crate) struct Export;

impl Tool for Export {
    fn name(&self) -> &'static str {
        "synth_export"
    }

    fn description(&self) -> &'static str {
        "Write a song recipe out as a Standard MIDI File, to open in a DAW — \
         synth_import the other way round. What is written is what the song \
         plays: the arrangement once with its transposes and mutes, chords and \
         step strings as their notes, swing and articulations applied, and the \
         tempo map, a ramp as a step every sixteenth note since MIDI has only \
         jumps. One MIDI track per song track, each on its own channel, named \
         for it. The sounds are not written — the file is the score — and a \
         song cannot say which tracks are drums, so name them in `drums` to put \
         them on General MIDI's kit. The reply names what the file could not \
         carry (humanize, fit, glides, microtonal pitches). The project is not \
         changed. Costs nothing."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "asset": {
                    "type": "string",
                    "description": "The synth_audio asset whose song recipe to write. \
                                    A one-shot has no notes in time and is refused."
                },
                "out": {
                    "type": "string",
                    "description": "Where to write the .mid — relative to the project \
                                    directory, or absolute. Omit and it lands in \
                                    cache/midi/<asset>.mid: rebuildable from the \
                                    recipe, so it is not kept as an asset."
                },
                "drums": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Tracks to write on channel 10, General MIDI's drum \
                                    kit, each as \"track\" to keep every note's key, or \
                                    \"track=key\" to play every note of it on that one \
                                    key: \"kick=36\", \"snare=38\", \"hat=42\". An \
                                    imported file's drum part is already a kit, so \
                                    it is named bare: \"drums\". Omit and every track \
                                    is a pitched part."
                }
            },
            "required": ["project", "asset"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let id = AssetId::new(text(arguments, "asset")?);
        let drums = drums(arguments)?;
        let out = under(&dir, arguments, "out")?;
        let project = load(&dir)?;
        let mut exported = synth::export_midi(&project, &dir, &id, &drums, out.as_deref())
            .map_err(|error| format!("{error}"))?;
        // Said back in the caller's own words, which resolve from the project
        // the way every path given to this server does.
        if let Some(given) = arguments.get("out").and_then(Value::as_str) {
            given.clone_into(&mut exported.shown);
        }
        let mut lines = vec![format!("{id} — written as MIDI to {}", exported.shown)];
        lines.extend(exported.lines());
        Ok(lines.join("\n").into())
    }
}

/// The `drums` argument, each entry read the way the command line's
/// `--drum` is.
fn drums(arguments: &Value) -> Result<Vec<Drum>, String> {
    let Some(given) = arguments.get("drums") else {
        return Ok(Vec::new());
    };
    let wrong = || "`drums` is a list of strings: \"kick=36\", \"drums\"".to_owned();
    given
        .as_array()
        .ok_or_else(wrong)?
        .iter()
        .map(|entry| entry.as_str().ok_or_else(wrong)?.parse::<Drum>())
        .collect()
}
