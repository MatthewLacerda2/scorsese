//! Putting a clip on a track, spoken in seconds.

use scorsese_core::{AssetId, ClipId, Placement, TrackId, placing};
use serde_json::Value;

use super::{bounds, frames, named, seconds};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Write one clip onto a track.
pub(crate) struct PlaceClip;

impl Tool for PlaceClip {
    fn name(&self) -> &'static str {
        "place_clip"
    }

    fn description(&self) -> &'static str {
        "Put a clip on a track: which asset, which track, when it starts and how \
         long it runs — all in seconds, rounded onto the project's frame grid for \
         you. This is the edit you would otherwise make by hand through \
         project_read and project_write, doing the frame arithmetic in your head; \
         that arithmetic is the one mistake that reaches the finished video \
         silently, because a window half a second off validates perfectly and is \
         only ever caught by watching. Leave `duration_seconds` out to run the \
         whole of the source from wherever you opened it. Nothing is written at \
         all unless the result is a document that still loads — a clip landing on \
         one already there, or a window reaching past the end of the media, is \
         refused with the reason and the project is left exactly as it was."
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
                    "description": "Id of the asset the clip shows — from the assets \
                                    table, never a path. `project_assets` lists them."
                },
                "track": {
                    "type": "string",
                    "description": "Id of the track to put it on. Must already exist: \
                                    a track invented from a typo would take the clip \
                                    with it, so this refuses and names the tracks \
                                    there are. A visual asset needs a video track and \
                                    an audible one an audio track."
                },
                "start_seconds": {
                    "type": "number",
                    "description": "When the clip begins on the timeline, in seconds \
                                    from the head of the cut. Rounded to the nearest \
                                    whole frame on the project's grid."
                },
                "duration_seconds": {
                    "type": "number",
                    "description": "How long the clip runs on the timeline, in \
                                    seconds. Leave it out for the rest of the source \
                                    — everything from the in-point to the end of the \
                                    measured media — which is what `put this shot in` \
                                    means. An asset with no measured length (a title, \
                                    a still, a colour, a brief nobody has generated, \
                                    or a file nobody has probed) has no rest to take, \
                                    so there it is required."
                },
                "source_in_seconds": {
                    "type": "number",
                    "description": "How far into the source the clip opens, in \
                                    seconds. Default 0, the head of the media. This is \
                                    a time in the SOURCE, not on the timeline, and it \
                                    means the same thing whatever framerate the source \
                                    was shot at — the conform to the project's grid is \
                                    done here."
                },
                "clip": {
                    "type": "string",
                    "description": "What to call the clip. Optional: without it the id \
                                    comes from the asset's, suffixed until it is free, \
                                    and the reply says which one it wrote. An id \
                                    already in use is refused rather than reused."
                }
            },
            "required": ["project", "asset", "track", "start_seconds"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let fps = project.timeline_fps;

        let start = seconds(arguments, "start_seconds")?.ok_or_else(|| {
            "`start_seconds` is required: when the clip begins on the timeline".to_owned()
        })?;
        let duration = seconds(arguments, "duration_seconds")?;
        let source_in = seconds(arguments, "source_in_seconds")?.unwrap_or(0.0);

        let placement = Placement {
            asset: AssetId::new(named(arguments, "asset", "the id of the asset to show")?),
            track: TrackId::new(named(
                arguments,
                "track",
                "the id of the track to put it on",
            )?),
            start: fps.frames(start),
            duration: duration.map(|seconds| frames(seconds, fps.as_f64())),
            source_in: fps.frames(source_in),
            id: arguments
                .get("clip")
                .and_then(Value::as_str)
                .map(ClipId::new),
        };
        let clip = placing::place(&mut project, &placement)
            .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        Ok(format!(
            "`{}` placed on `{}`: {}.",
            clip.id,
            placement.track,
            bounds(fps, &clip)
        )
        .into())
    }
}
