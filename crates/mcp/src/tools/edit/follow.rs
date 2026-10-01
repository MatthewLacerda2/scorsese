//! Sending a clip along an arrow: the `follow` block and the ramp that moves
//! it, in one call.
//!
//! Two edits a person means as one — *this dot travels down that connector* —
//! so they are written together: a `follow` with no `follow.progress` track
//! sits at the arrow's tail forever, which is the mistake `project_check`
//! otherwise has to warn about after the fact.

use scorsese_core::{ClipId, TrackKind};
use scorsese_core::{Easing, Follow, Frames, Keyframe, KeyframeTrack, Project, PropertyPath};
use scorsese_render::picture::path::FOLLOW_PROGRESS;
use serde_json::Value;

use super::{named, seconds};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Set or clear the arrow a clip travels along.
pub(crate) struct ClipFollow;

impl Tool for ClipFollow {
    fn name(&self) -> &'static str {
        "clip_follow"
    }

    fn description(&self) -> &'static str {
        "Send a placed clip along an arrow: it travels the arrow clip's line from \
         tail to head — curves included — instead of in a straight line between \
         keyframed positions. The clip's middle rides the line, its own position \
         becomes an offset from there, and with `orient` it turns to face the way \
         the line runs. Writes the clip's `follow` and one ordinary \
         `follow.progress` ramp from 0 (the tail) to 1 (the head), which stays \
         editable afterwards with project_write — a pause half way, or a trip \
         back, is more keyframes on that track. The arrow is named by its CLIP \
         (the same arrow asset can be on screen twice), on the same timeline as \
         the clip, and need not be showing yet. `stop` removes the follow and its \
         ramp. Nothing is written unless the whole document still loads."
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
                    "description": "Id of the clip that travels. Picture only."
                },
                "arrow": {
                    "type": "string",
                    "description": "Id of the arrow CLIP whose line it travels along — \
                                    a clip showing an arrow shape asset, not the asset \
                                    itself. Required unless `stop` is true."
                },
                "orient": {
                    "type": "boolean",
                    "description": "Turn the clip to face along the line, as a car \
                                    follows a road: its own rotation is added on top. \
                                    Draw the clip pointing right for this to read \
                                    naturally. Default false: it slides along facing \
                                    however it was drawn."
                },
                "start_seconds": {
                    "type": "number",
                    "description": "When it sets off from the tail, in seconds from \
                                    the start of the CLIP (not the timeline). Default 0. \
                                    Before this it waits at the tail."
                },
                "travel_seconds": {
                    "type": "number",
                    "description": "How long it takes to reach the head, in seconds. \
                                    Default: the rest of the clip. After it arrives it \
                                    waits at the head."
                },
                "easing": {
                    "type": "string",
                    "description": "How it gathers and loses speed on the way: \
                                    `linear` (the default, an even pace), `ease_in`, \
                                    `ease_out`, `ease_in_out`, `back_out`, `spring` — \
                                    any keyframe easing name."
                },
                "stop": {
                    "type": "boolean",
                    "description": "Remove the clip's follow and its follow.progress \
                                    track, leaving it placed by its transform alone."
                }
            },
            "required": ["project", "clip"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let id = ClipId::new(named(arguments, "clip", "the id of the clip that travels")?);
        let said = follow(&mut project, &id, arguments)?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(format!("`{id}`: {said}.").into())
    }
}

/// Apply the request to clip `id` on a copy that becomes `project` only if it
/// validates; what was done, in words.
fn follow(project: &mut Project, id: &ClipId, arguments: &Value) -> Result<String, String> {
    let fps = project.timeline_fps;
    let mut proposed = project.clone();
    let clip = proposed
        .tracks
        .iter_mut()
        .filter(|track| track.kind == TrackKind::Video)
        .flat_map(|track| track.clips.iter_mut())
        .find(|clip| &clip.id == id)
        .ok_or_else(|| format!("there is no clip `{id}` on a video track in this project"))?;
    let progress = PropertyPath::new(FOLLOW_PROGRESS);
    clip.keyframes.retain(|track| track.property != progress);

    let said = if arguments.get("stop").and_then(Value::as_bool) == Some(true) {
        clip.follow = None;
        "no longer follows an arrow, and its follow.progress track is gone".to_owned()
    } else {
        let arrow = ClipId::new(named(arguments, "arrow", "the id of the arrow clip")?);
        let orient = arguments.get("orient").and_then(Value::as_bool) == Some(true);
        let easing: Easing = match arguments.get("easing").filter(|e| !e.is_null()) {
            None => Easing::Linear,
            Some(name) => serde_json::from_value(name.clone())
                .map_err(|_| format!("`easing` {name} is not a keyframe easing name"))?,
        };
        let at = fps.frames(seconds(arguments, "start_seconds")?.unwrap_or(0.0));
        let over = match seconds(arguments, "travel_seconds")? {
            Some(travel) => fps.frames(travel),
            None => Frames(clip.duration.get().saturating_sub(at.get())),
        };
        if over == Frames::ZERO {
            return Err(
                "the trip takes no frames — give a travel_seconds, or start it \
                        before the clip ends"
                    .to_owned(),
            );
        }
        if at + over > clip.duration {
            return Err(format!(
                "it would arrive at frame {} of the clip, which is only {} frames long — \
                 nothing was written",
                (at + over).get(),
                clip.duration.get()
            ));
        }
        clip.follow = Some(Follow {
            clip: arrow.clone(),
            orient,
        });
        clip.keyframes.push(KeyframeTrack::new(
            progress,
            vec![
                Keyframe {
                    t: at,
                    value: 0.0,
                    easing,
                },
                Keyframe {
                    t: at + over,
                    value: 1.0,
                    easing: Easing::Linear,
                },
            ],
        ));
        format!(
            "follows arrow `{arrow}`{}, leaving its tail {:.2}s into the clip and reaching \
             its head {:.2}s later",
            if orient { ", facing along it" } else { "" },
            fps.seconds(at),
            fps.seconds(over)
        )
    };
    proposed.validate().map_err(|errors| {
        let problems: Vec<String> = errors.into_vec().iter().map(ToString::to_string).collect();
        format!("{} — nothing was written", problems.join("; "))
    })?;
    *project = proposed;
    Ok(said)
}
