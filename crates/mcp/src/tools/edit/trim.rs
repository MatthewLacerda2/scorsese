//! Moving a placed clip, or changing what part of its source it shows.

use scorsese_core::{ClipId, Trim, placing};
use serde_json::Value;

use super::{bounds, frames, named, seconds};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Change where a placed clip starts, how long it runs, or where it opens.
pub(crate) struct TrimClip;

impl Tool for TrimClip {
    fn name(&self) -> &'static str {
        "trim_clip"
    }

    fn description(&self) -> &'static str {
        "Move a clip already on the timeline, or change how long it runs or where \
         in its source it opens — in seconds, rounded onto the project's frame \
         grid. The counterpart to place_clip and the other half of assembling a \
         cut: nudging a line of narration to land on a beat, tightening a shot \
         that runs long, dropping the first second of a take. Each argument sets \
         that field and only that field, so a start on its own MOVES the clip and \
         a duration on its own changes where it ends. Nothing is written at all \
         unless the result is a document that still loads — a clip moved onto its \
         neighbour, or trimmed past the end of its media, is refused with the \
         reason and the project is left exactly as it was."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "clip": {
                    "type": "string",
                    "description": "Id of the clip to change. It stays on the track it \
                                    is on — moving a clip between tracks is a \
                                    project_write, because which track a clip sits on \
                                    is what decides what is drawn over what."
                },
                "start_seconds": {
                    "type": "number",
                    "description": "Where the clip begins on the timeline, in seconds \
                                    from the head of the cut. On its own this moves \
                                    the clip and leaves its length and its source \
                                    window alone. Left out, the clip stays where it is."
                },
                "duration_seconds": {
                    "type": "number",
                    "description": "How long the clip runs on the timeline, in \
                                    seconds. On its own this holds the start and moves \
                                    the end. Left out, the length is unchanged."
                },
                "source_in_seconds": {
                    "type": "number",
                    "description": "How far into the source the clip opens, in \
                                    seconds — a time in the SOURCE, not on the \
                                    timeline. On its own this shows a later part of \
                                    the media in the same slot, which is what dropping \
                                    a slow first second of a take means. Left out, the \
                                    in-point is unchanged."
                }
            },
            "required": ["project", "clip"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let fps = project.timeline_fps;
        let id = ClipId::new(named(arguments, "clip", "the id of the clip to change")?);

        let bounds_asked = Trim {
            start: seconds(arguments, "start_seconds")?.map(|at| fps.frames(at)),
            duration: seconds(arguments, "duration_seconds")?
                .map(|length| frames(length, fps.as_f64())),
            source_in: seconds(arguments, "source_in_seconds")?.map(|at| fps.frames(at)),
        };
        let clip = placing::trim(&mut project, &id, &bounds_asked)
            .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        Ok(format!("`{}` now {}.", clip.id, bounds(fps, &clip)).into())
    }
}
