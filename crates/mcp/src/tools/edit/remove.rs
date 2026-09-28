//! Taking placed clips off the timeline.

use std::collections::BTreeSet;

use scorsese_core::{ClipId, placing};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Remove clips from their tracks, leaving their assets and the gap.
pub(crate) struct ClipRemove;

impl Tool for ClipRemove {
    fn name(&self) -> &'static str {
        "clip_remove"
    }

    fn description(&self) -> &'static str {
        "Take clips off the timeline by id, leaving the assets they showed and \
         every other clip exactly where they are. A clip is only a placement, so \
         nothing else goes with it: its asset stays in the assets table and its \
         file on disk, ready to be placed again, and the gap it leaves is not \
         closed up. All or nothing: an id that names no clip, or a clip an arrow is still \
         attached to, refuses the whole request with the reason and removes \
         nothing. This is the edit to make instead of rewriting the document with \
         project_write."
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
                    "description": "Ids of the clips to remove — one or several, \
                                    removed together or not at all. project_describe \
                                    and project_read name them."
                }
            },
            "required": ["project", "clips"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let fps = project.timeline_fps;
        let removed = placing::remove(&mut project, &clip_ids(arguments)?)
            .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        let each: Vec<String> = removed
            .iter()
            .map(|gone| {
                format!(
                    "`{}` from `{}` ({:.2}s–{:.2}s, showing `{}`)",
                    gone.clip.id,
                    gone.track,
                    fps.seconds(gone.clip.start),
                    fps.seconds(gone.clip.end()),
                    gone.clip.asset
                )
            })
            .collect();
        Ok(format!(
            "Removed {}. The assets are still in the project, and nothing else moved.",
            each.join("; ")
        )
        .into())
    }
}

/// The clips to remove, as a set — a repeated id is one clip, not two.
fn clip_ids(arguments: &Value) -> Result<BTreeSet<ClipId>, String> {
    let named = arguments
        .get("clips")
        .and_then(Value::as_array)
        .ok_or_else(|| "`clips` is required: the ids of the clips to remove".to_owned())?;
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
