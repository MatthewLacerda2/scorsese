//! Moving a placed clip onto another track.

use scorsese_core::{ClipId, Relocation, TrackId, placing};
use serde_json::Value;

use super::{bounds, named, seconds};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Move one clip to another track, and optionally to a new start on it.
pub(crate) struct ClipMove;

impl Tool for ClipMove {
    fn name(&self) -> &'static str {
        "clip_move"
    }

    fn description(&self) -> &'static str {
        "Move a clip already on the timeline onto another track — optionally to a \
         new start there in the same edit, the way dragging it down a lane and \
         along it is one gesture. Which video track a clip is on decides what is \
         drawn over what (the first track is at the bottom), so this is how a \
         picture goes in front of or behind another; between audio tracks it only \
         changes which lane it is mixed from. Everything else about the clip goes \
         with it: its id, its source window, its speed and its keyframes. It keeps \
         place_clip's rules — the track must exist, picture goes on a video track \
         and sound on an audio one, and it may not land on a clip already there — \
         and nothing is written at all unless the result is a document that still \
         loads. To move a clip in time on the track it is on, use trim_clip."
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
                    "description": "Id of the clip to move. project_describe and \
                                    project_read name them."
                },
                "track": {
                    "type": "string",
                    "description": "Id of the track to move it onto. Must already \
                                    exist and carry the clip's kind — picture onto a \
                                    video track, sound onto an audio one; a missing \
                                    track is refused, naming the tracks there are."
                },
                "start_seconds": {
                    "type": "number",
                    "description": "Where the clip begins on its new track, in \
                                    seconds from the head of the cut, rounded to the \
                                    nearest frame. Left out, it keeps the start it \
                                    has — lifted straight up or down a lane."
                }
            },
            "required": ["project", "clip", "track"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let fps = project.timeline_fps;
        let id = ClipId::new(named(arguments, "clip", "the id of the clip to move")?);
        let to = Relocation {
            track: TrackId::new(named(
                arguments,
                "track",
                "the id of the track to move it onto",
            )?),
            start: seconds(arguments, "start_seconds")?.map(|at| fps.frames(at)),
        };
        let from = project
            .clips()
            .find(|(_, clip)| clip.id == id)
            .map(|(track, _)| track.id.clone());
        let clip = placing::relocate(&mut project, &id, &to)
            .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        let from = from.map_or_else(String::new, |track| format!(" from `{track}`"));
        Ok(format!(
            "`{}` moved{from} to `{}`: {}.",
            clip.id,
            to.track,
            bounds(fps, &clip)
        )
        .into())
    }
}
