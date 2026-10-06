//! Moving a placed clip: along its track, onto another, or within its source.

use schemars::JsonSchema;
use scorsese_core::{ClipId, Relocation, TrackId, Trim, placing};
use serde::Deserialize;
use serde_json::Value;

use super::{bounds, frames, seconds};
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Change where a placed clip starts, how long it runs, where it opens in its
/// source, or which track it is on — one tool for one gesture (#781).
pub(crate) struct ClipMove;

/// What `clip_move` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the clip to move. project_describe and project_read name them.
    clip: Name,
    /// Where the clip begins on the timeline, in seconds from the head of the
    /// cut. On its own this moves the clip and leaves its length and its source
    /// window alone. Left out, the clip keeps the start it has.
    start_seconds: Option<f64>,
    /// How long the clip runs on the timeline, in seconds. On its own this
    /// holds the start and moves the end. Left out, the length is unchanged.
    duration_seconds: Option<f64>,
    /// How far into the source the clip opens, in seconds — a time in the
    /// SOURCE, not on the timeline. On its own this shows a later part of the
    /// media in the same slot, which is what dropping a slow first second of a
    /// take means. Left out, the in-point is unchanged.
    source_in_seconds: Option<f64>,
    /// Id of a track to move the clip onto. Must already exist and carry the
    /// clip's kind — picture onto a video track, sound onto an audio one; a
    /// missing track is refused, naming the tracks there are. Which video track
    /// a clip is on decides what is drawn over what (the first track is at the
    /// bottom). Left out, the clip stays on the track it is on.
    track: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("clip", "the id of the clip to move")];
}

impl Tool for ClipMove {
    fn name(&self) -> &'static str {
        "clip_move"
    }

    fn description(&self) -> &'static str {
        "Move a clip already on the timeline — along its track, onto another \
         track, or to a new length or in-point in its source. Times are in \
         seconds, rounded onto the project's frame grid, and a track goes with \
         them in the same edit. The counterpart to place_clip and the other half of assembling a cut: \
         nudging a line of narration to land on a beat, tightening a shot that \
         runs long, dropping the first second of a take, putting a picture in \
         front of another. Each argument sets that field and only that field, so \
         a start on its own MOVES the clip, a duration on its own changes where \
         it ends, and a track on its own keeps its timing. Everything else about \
         the clip stays: its id, its speed and its keyframes. A track keeps \
         place_clip's rules — it must exist, picture goes on a video track and \
         sound on an audio one. Nothing is written at all unless the result is a \
         document that still loads — a clip moved onto its neighbour, or trimmed \
         past the end of its media, is refused with the reason and the project is \
         left exactly as it was."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let fps = project.timeline_fps;
        let id = ClipId::new(arguments.clip.as_str());

        let bounds_asked = Trim {
            start: seconds(arguments.start_seconds, "start_seconds")?.map(|at| fps.frames(at)),
            duration: seconds(arguments.duration_seconds, "duration_seconds")?
                .map(|length| frames(length, fps.as_f64())),
            source_in: seconds(arguments.source_in_seconds, "source_in_seconds")?
                .map(|at| fps.frames(at)),
        };
        let from = project
            .clips()
            .find(|(_, clip)| clip.id == id)
            .map(|(track, _)| track.id.clone());
        let moved = match args::given(arguments.track.as_deref()) {
            // A track and new bounds are one edit, never a relocation then a
            // trim: the document in between can overlap where the result does
            // not (`Relocation`'s doc).
            Some(track) => {
                let to = Relocation {
                    track: TrackId::new(track),
                    bounds: bounds_asked,
                };
                placing::relocate(&mut project, &id, &to)
                    .map(|clip| (clip, Some(to.track)))
                    .map_err(|error| error.to_string())
            }
            None if bounds_asked == Trim::default() => Err(
                "nothing to change — say a new start, duration, source in-point or track"
                    .to_owned(),
            ),
            None => placing::trim(&mut project, &id, &bounds_asked)
                .map(|clip| (clip, None))
                .map_err(|error| error.to_string()),
        };
        let (clip, onto) = moved.map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        Ok(match onto.filter(|onto| from.as_ref() != Some(onto)) {
            Some(onto) => {
                let from = from.map_or_else(String::new, |track| format!(" from `{track}`"));
                format!(
                    "`{}` moved{from} to `{onto}`: {}.",
                    clip.id,
                    bounds(fps, &clip)
                )
            }
            None => format!("`{}` now {}.", clip.id, bounds(fps, &clip)),
        }
        .into())
    }
}
