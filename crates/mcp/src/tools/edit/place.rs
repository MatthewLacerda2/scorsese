//! Putting a clip on a track, spoken in seconds.

use schemars::JsonSchema;
use scorsese_core::{AssetId, ClipId, Fps, Frames, Placement, TrackId, placing};
use serde::Deserialize;
use serde_json::Value;

use super::{bounds, frames, seconds};
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Write one clip onto a track.
pub(crate) struct PlaceClip;

/// What `place_clip` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the asset the clip shows — from the assets table, never a path.
    /// `project_assets` lists them.
    asset: Name,
    /// Id of the track to put it on. Must already exist: a track invented from
    /// a typo would take the clip with it, so this refuses and names the tracks
    /// there are. A visual asset needs a video track and an audible one an
    /// audio track.
    track: Name,
    /// For sound effects that overlap: when the clip would land on one already
    /// on `track`, put it on the first of `track-2`, `track-3`… with room
    /// instead, making the next one when none has. The reply names the track
    /// it landed on. Audio tracks only, and `track` itself must exist. Default
    /// false: an overlap is refused.
    spill_over: Option<bool>,
    /// When the clip begins on the timeline, in seconds from the head of the
    /// cut. Rounded to the nearest whole frame on the project's grid.
    start_seconds: f64,
    /// How long the clip runs on the timeline, in seconds. Leave it out for the
    /// rest of the source — everything from the in-point to the end of the
    /// measured media — which is what `put this shot in` means. An asset with
    /// no measured length (a title, a still, a colour, a brief nobody has
    /// generated, or a file nobody has probed) has no rest to take, so there it
    /// is required.
    duration_seconds: Option<f64>,
    /// How far into the source the clip opens, in seconds. Default 0, the head
    /// of the media. This is a time in the SOURCE, not on the timeline, and it
    /// means the same thing whatever framerate the source was shot at — the
    /// conform to the project's grid is done here.
    source_in_seconds: Option<f64>,
    /// What to call the clip. Optional: without it the id comes from the
    /// asset's, suffixed until it is free, and the reply says which one it
    /// wrote. An id already in use is refused rather than reused.
    clip: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[
        ("asset", "the id of the asset to show"),
        ("track", "the id of the track to put it on"),
        ("start_seconds", "when the clip begins on the timeline"),
    ];
}

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
         refused with the reason and the project is left exactly as it was. \
         For sound effects that overlap each other, set `spill_over` and each one \
         lands on the first free lane of the track's set (`sfx`, `sfx-2`, …)."
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

        let start = seconds(Some(arguments.start_seconds), "start_seconds")?.unwrap_or_default();
        let duration = seconds(arguments.duration_seconds, "duration_seconds")?;
        let source_in = seconds(arguments.source_in_seconds, "source_in_seconds")?.unwrap_or(0.0);

        let placement = Placement {
            asset: AssetId::new(arguments.asset.as_str()),
            track: TrackId::new(arguments.track.as_str()),
            start: fps.frames(start),
            duration: duration.map(|seconds| reaching(fps, start, seconds)),
            source_in: fps.frames(source_in),
            id: arguments.clip.as_deref().map(ClipId::new),
        };
        let (track, clip) = if arguments.spill_over.unwrap_or(false) {
            placing::place_spilling(&mut project, &placement)
        } else {
            placing::place(&mut project, &placement).map(|clip| (placement.track.clone(), clip))
        }
        .map_err(|error| format!("{error} — nothing was written"))?;
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        Ok(format!(
            "`{}` placed on `{}`: {}.",
            clip.id,
            track,
            bounds(fps, &clip)
        )
        .into())
    }
}

/// How many frames a clip covers from `start` for `seconds`, found by putting
/// its **end** on the grid rather than its length.
///
/// Rounding the start and the length separately can land a clip's end a frame
/// past where rounding its end would have put it — so two clips placed back to
/// back on the same boundary, one ending where the next begins, overlapped by
/// a frame and the second was refused. That is exactly how section bounds are
/// used: a caption per section, each ending where the next one starts. With
/// the end rounded the same way the next clip's start is, the same number of
/// seconds always lands on the same frame, whichever side of it a clip is on.
fn reaching(fps: Fps, start: f64, seconds: f64) -> Frames {
    let length = frames(seconds, fps.as_f64());
    if length == Frames::ZERO {
        return length;
    }
    let (from, to) = (fps.frames(start), fps.frames(start + seconds));
    Frames(to.get().saturating_sub(from.get()).max(1))
}
