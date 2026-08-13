//! Dissolving one shot into the next.

use scorsese_core::ClipId;
use scorsese_render::dissolve;
use serde_json::Value;

use super::{clip_id, frames, number};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Dissolve one shot into the next.
pub(crate) struct Dissolve;

impl Tool for Dissolve {
    fn name(&self) -> &'static str {
        "dissolve"
    }

    fn description(&self) -> &'static str {
        "Dissolve one shot into the next, by writing ordinary opacity keyframes \
         on both clips — the same ones you would place by hand, and they stay \
         editable afterwards. Two clips on one track may not overlap and a \
         crossover needs them to, so the incoming clip is MOVED to a track above \
         and pulled back over the outgoing one; the reply says what that \
         rearranged. The clips must currently meet at a cut and each must be at \
         least as long as the crossover, or nothing is changed at all."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "from": {
                    "type": "string",
                    "description": "Id of the outgoing clip — the shot being left. \
                                    It fades out over its own last frames and stays \
                                    where it is."
                },
                "to": {
                    "type": "string",
                    "description": "Id of the incoming clip — the shot arriving. It \
                                    fades in, moves to a track above, and starts \
                                    earlier than it did by the length of the \
                                    crossover."
                },
                "seconds": {
                    "type": "number",
                    "description": "How long the crossover lasts. Default 0.5. \
                                    Rounded to whole frames on the project's grid \
                                    and never to zero — a crossover of no length is \
                                    a cut, and asking for one is refused."
                }
            },
            "required": ["project", "from", "to"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let from = clip_id(arguments, "from")?;
        let to = clip_id(arguments, "to")?;

        let seconds = number(arguments, "seconds", 0.5);
        let duration = frames(seconds, project.timeline_fps.as_f64());
        let placed = dissolve(&mut project, &ClipId::new(from), &ClipId::new(to), duration)
            .map_err(|error| format!("{error} — nothing was changed"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        let made = if placed.track_created { " (new)" } else { "" };
        Ok(format!(
            "`{from}` dissolves into `{to}` over {} frames — ordinary opacity \
             keyframes, editable and deletable. `{to}` moved to track `{}`{made} \
             and now starts {} frames earlier.",
            duration.get(),
            placed.track,
            placed.pulled_back
        )
        .into())
    }
}
