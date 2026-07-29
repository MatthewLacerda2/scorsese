//! Lowering the music while narration plays.

use scorsese_core::{Dip, PropertyPath, TrackId, Under, duck_track};
use scorsese_render::audio::path::VOLUME;
use serde_json::Value;

use super::{frames, number};
use crate::tools::inspect::load;
use crate::tools::{Tool, project_dir, project_property};

/// Lower the music under narration.
pub(crate) struct Duck;

impl Tool for Duck {
    fn name(&self) -> &'static str {
        "duck_music"
    }

    fn description(&self) -> &'static str {
        "Lower a music track while narration plays over it, by writing ordinary \
         volume keyframes on its clips. Safe to run repeatedly: it replaces only \
         the keyframes it wrote and never touches ones set by hand. Works on \
         narration that has not been generated yet, since it triggers on where \
         the clips are rather than on the sound."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "music": {
                    "type": "string",
                    "description": "Id of the audio track to duck — the music."
                },
                "depth": {
                    "type": "number",
                    "description": "How far down, as a multiplier on the clip's own \
                                    level. 0.25 is a quarter as loud. Default 0.25."
                },
                "attack_seconds": {
                    "type": "number",
                    "description": "Seconds to reach the ducked level. The dip is \
                                    fully down by the moment narration starts. \
                                    Default 0.3."
                },
                "release_seconds": {
                    "type": "number",
                    "description": "Seconds to come back up. Longer than the attack \
                                    on purpose — returning early lurches. Default 0.6."
                },
                "under": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Track ids that count as narration. Omit and every \
                                    other audio track does."
                }
            },
            "required": ["project", "music"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<String, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let music = arguments
            .get("music")
            .and_then(Value::as_str)
            .ok_or_else(|| "`music` is required: the track id to duck".to_owned())?;

        let fps = project.timeline_fps.as_f64();
        let dip = Dip {
            under: number(arguments, "depth", 0.25),
            over: 1.0,
            attack: frames(number(arguments, "attack_seconds", 0.3), fps),
            release: frames(number(arguments, "release_seconds", 0.6), fps),
        };
        let under = Under {
            music: TrackId::new(music),
            narration: arguments
                .get("under")
                .and_then(Value::as_array)
                .map(|ids| {
                    ids.iter()
                        .filter_map(Value::as_str)
                        .map(TrackId::new)
                        .collect()
                })
                .unwrap_or_default(),
        };

        let report = duck_track(&mut project, &under, &PropertyPath::new(VOLUME), dip)
            .ok_or_else(|| format!("no track `{music}` in this project"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(format!(
            "{} clip(s) ducked, {} left alone — ordinary volume keyframes, \
             editable and deletable",
            report.dipped.len(),
            report.untouched.len()
        ))
    }
}
