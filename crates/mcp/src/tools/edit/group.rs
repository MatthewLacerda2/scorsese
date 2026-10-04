//! Wrapping clips into a group, and a group back into clips.

use schemars::JsonSchema;
use scorsese_core::grouping::{self, Grouping};
use scorsese_core::{AssetId, ClipId, TrackId};
use serde::Deserialize;
use serde_json::Value;

use super::clip_set;
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Wrap clips already on the timeline into a group that renders as one layer.
pub(crate) struct ClipGroup;

/// What `clip_group` takes.
#[derive(Deserialize, JsonSchema)]
struct GroupArguments {
    project: ProjectDir,
    /// Ids of the clips to group — every one on one of the project's own video
    /// tracks. They may be on different tracks and at different times; the
    /// group runs from the earliest start to the latest end. project_describe
    /// and project_read name them.
    clips: Vec<String>,
    /// Id for the new group asset. Left out, it is `group`, or `group-2` and so
    /// on if that is taken. Name it for what it is — `pipeline-diagram` — since
    /// it is how the group is found again.
    asset: Option<String>,
    /// Id for the one clip that shows the group. Left out, it is `c-` followed
    /// by the asset's id.
    clip: Option<String>,
    /// Id of the video track to put the group clip on. Left out, it is the
    /// lowest track any of the grouped clips came from, so nothing that was
    /// under them ends up over the group.
    track: Option<String>,
}

impl args::Arguments for GroupArguments {
    const REQUIRED: Required = &[("clips", "the ids of the clips to group")];
}

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
        args::schema::<GroupArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: GroupArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let fps = project.timeline_fps;
        let request = Grouping {
            clips: clip_set(&arguments.clips)?,
            asset: args::given(arguments.asset.as_deref()).map(AssetId::new),
            clip: args::given(arguments.clip.as_deref()).map(ClipId::new),
            track: args::given(arguments.track.as_deref()).map(TrackId::new),
        };
        let grouped = grouping::group(&mut project, &request)
            .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(dir)
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

/// What `clip_ungroup` takes.
#[derive(Deserialize, JsonSchema)]
struct UngroupArguments {
    project: ProjectDir,
    /// Id of the clip showing the group — the one clip_group made, or any clip
    /// of a group asset on the project's own tracks.
    clip: Name,
}

impl args::Arguments for UngroupArguments {
    const REQUIRED: Required = &[("clip", "the id of the group's clip")];
}

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
        args::schema::<UngroupArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: UngroupArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let clip = ClipId::new(arguments.clip.as_str());
        let back = grouping::ungroup(&mut project, &clip)
            .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(dir)
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
