//! A lane to put clips on.

use scorsese_core::{Lane, TrackKind, authoring};
use serde_json::Value;

use super::{maybe, refused, save, wanted};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Add a track.
pub(crate) struct TrackNew;

impl Tool for TrackNew {
    fn name(&self) -> &'static str {
        "track_new"
    }

    fn description(&self) -> &'static str {
        "Add a track — a lane for clips, carrying either picture or sound. This \
         is the answer to the commonest refusal in the whole tool: clips on one \
         track may not overlap, so anything meant to be on screen at the same \
         time as something else needs a lane of its own. A new video track is \
         appended, which means it composites OVER everything already there — \
         which is what a caption above footage wants. Audio tracks are unordered \
         among themselves; everything audible is summed. Left unnamed it is \
         numbered `v1`, `v2`, `a1`, `a2` from the lowest free number. Reordering \
         the layers afterwards is a project_write."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "kind": {
                    "type": "string",
                    "enum": ["video", "audio"],
                    "description": "What the lane carries. `video` takes anything \
                                    visible — footage, a title, a colour, a shape, an \
                                    icon, a generated shot; `audio` takes sound. An \
                                    asset on the wrong kind of track is refused."
                },
                "track": {
                    "type": "string",
                    "description": "What to call it. Optional: without it the lane is \
                                    numbered from the lowest free `v`/`a` number, and \
                                    the reply says which id it wrote. An id already in \
                                    use is refused."
                },
                "name": {
                    "type": "string",
                    "description": "What a human calls the lane, shown in a lane header. \
                                    Cosmetic — nothing reads it and nothing renders it."
                },
                "note": {
                    "type": "string",
                    "description": "Why this lane is here, for whoever reads the project \
                                    next: why the music sits under the narration, which \
                                    one gets ducked. Never rendered, under any setting."
                }
            },
            "required": ["project", "kind"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let kind = match maybe(arguments, "kind").as_deref() {
            Some("video") => TrackKind::Video,
            Some("audio") => TrackKind::Audio,
            Some(other) => return Err(format!("`kind` is video or audio, not `{other}`")),
            None => return Err("`kind` is required: video or audio".to_owned()),
        };
        let lane = Lane {
            kind,
            id: wanted(arguments, "track").map(scorsese_core::TrackId::new),
            name: maybe(arguments, "name"),
            note: maybe(arguments, "note"),
        };
        let id = authoring::add_track(&mut project, &lane).map_err(refused)?;
        save(&project, &dir)?;
        let (says, where_it_sits) = match kind {
            TrackKind::Video => ("video", "over every video track already there"),
            TrackKind::Audio => ("audio", "mixed with every other audio track"),
        };
        Ok(format!("`{id}` — a {says} track, {where_it_sits}. place_clip fills it.").into())
    }
}
