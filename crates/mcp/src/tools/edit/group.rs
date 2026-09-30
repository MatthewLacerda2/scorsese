//! Wrapping clips into a group, and a group back into clips.

use std::collections::BTreeSet;

use scorsese_core::grouping::{self, Grouping};
use scorsese_core::{AssetId, ClipId, TrackId};
use serde_json::Value;

use super::named;
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Wrap clips already on the timeline into a group that renders as one layer.
pub(crate) struct ClipGroup;

impl Tool for ClipGroup {
    fn name(&self) -> &'static str {
        "clip_group"
    }

    fn description(&self) -> &'static str {
        "Group clips already on the timeline so they render as one layer — what \
         Filmora calls a compound clip. The clips move into a new group asset and \
         are replaced by one clip of it at the same place and time, so the picture \
         does not change; from then on that one clip can be moved, scaled, faded or \
         blurred as a unit, with clip_set, trim_clip, keyframes and every other \
         tool, the way one clip is. A diagram of thirty boxes and arrows that pulls \
         back as a whole is thirty clips grouped and two keyframes on the group, \
         rather than the same move written thirty times. Faded, a group fades as \
         one picture: its overlapping members do not show through each other. \
         Picture only — every clip must be on a video track. Each track the clips \
         came from becomes one of the group's own tracks, in the same order, and \
         the group clip goes on the lowest of them unless `track` names another. \
         Every clip keeps its id and its keyframes. An arrow attached to a grouped \
         clip must be grouped too (and one inside may follow only clips inside): \
         otherwise nothing is written and the reply names the arrow. Undo with \
         clip_ungroup."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "clips": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Ids of the clips to group — every one on one of \
                                    the project's own video tracks. They may be on \
                                    different tracks and at different times; the \
                                    group runs from the earliest start to the latest \
                                    end. project_describe and project_read name them."
                },
                "asset": {
                    "type": "string",
                    "description": "Id for the new group asset. Left out, it is \
                                    `group`, or `group-2` and so on if that is taken. \
                                    Name it for what it is — `pipeline-diagram` — \
                                    since it is how the group is found again."
                },
                "clip": {
                    "type": "string",
                    "description": "Id for the one clip that shows the group. Left \
                                    out, it is `c-` followed by the asset's id."
                },
                "track": {
                    "type": "string",
                    "description": "Id of the video track to put the group clip on. \
                                    Left out, it is the lowest track any of the \
                                    grouped clips came from, so nothing that was \
                                    under them ends up over the group."
                }
            },
            "required": ["project", "clips"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let fps = project.timeline_fps;
        let request = Grouping {
            clips: clip_ids(arguments)?,
            asset: optional(arguments, "asset").map(AssetId::new),
            clip: optional(arguments, "clip").map(ClipId::new),
            track: optional(arguments, "track").map(TrackId::new),
        };
        let grouped = grouping::group(&mut project, &request)
            .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        let lanes: Vec<String> = grouped.tracks.iter().map(|t| format!("`{t}`")).collect();
        Ok(format!(
            "Grouped {} clip(s) into `{}`, shown by clip `{}` on track `{}` from {:.2}s for \
             {:.2}s. Inside, they sit on the group's own tracks {}, timed from the \
             group's start. Animate or trim `{}` to move them as one.",
            request.clips.len(),
            grouped.asset,
            grouped.clip,
            grouped.track,
            fps.seconds(grouped.start),
            fps.seconds(grouped.duration),
            lanes.join(", "),
            grouped.clip
        )
        .into())
    }
}

/// Put a group's clips back on the timeline, where the group showed them.
pub(crate) struct ClipUngroup;

impl Tool for ClipUngroup {
    fn name(&self) -> &'static str {
        "clip_ungroup"
    }

    fn description(&self) -> &'static str {
        "Undo clip_group: replace a clip of a group with the group's own clips, at \
         exactly the times and places the group was showing them, and remove the \
         group asset. The group's tracks come back as tracks directly above the one \
         the group clip was on, keeping their order and every id, so arrows still \
         follow what they followed. What the group clip did to the group as a \
         whole — a move, a scale, a fade, a blur — applied to one layer and is not \
         carried onto the members; the reply says when there was something to \
         lose. Refused, with nothing written, when the group is placed more than \
         once (ungrouping one copy would have to duplicate it) or when the clip \
         shows only part of its group (trim it back to the whole group first, so \
         no member comes out cut)."
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
                    "description": "Id of the clip showing the group — the one \
                                    clip_group made, or any clip of a group asset on \
                                    the project's own tracks."
                }
            },
            "required": ["project", "clip"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let clip = ClipId::new(named(arguments, "clip", "the id of the group's clip")?);
        let back = grouping::ungroup(&mut project, &clip)
            .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        let lanes: Vec<String> = back.tracks.iter().map(|t| format!("`{t}`")).collect();
        let lost = if back.dropped_its_own_look {
            format!(
                " Clip `{clip}` carried keyframes or effects of its own, which applied to \
                 the group as one layer and were not carried onto the clips."
            )
        } else {
            String::new()
        };
        Ok(format!(
            "Ungrouped `{}`: {} clip(s) are back on the timeline, on tracks {}, and the \
             group asset `{}` is gone.{lost}",
            clip,
            back.clips,
            lanes.join(", "),
            back.asset
        )
        .into())
    }
}

/// The clips to group, as a set — a repeated id is one clip.
fn clip_ids(arguments: &Value) -> Result<BTreeSet<ClipId>, String> {
    let named = arguments
        .get("clips")
        .and_then(Value::as_array)
        .ok_or_else(|| "`clips` is required: the ids of the clips to group".to_owned())?;
    let ids: BTreeSet<ClipId> = named
        .iter()
        .filter_map(Value::as_str)
        .map(ClipId::new)
        .collect();
    if ids.is_empty() {
        return Err("`clips` must name at least one clip".to_owned());
    }
    Ok(ids)
}

/// An optional string argument, `None` when absent or blank.
fn optional<'a>(arguments: &'a Value, key: &str) -> Option<&'a str> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
