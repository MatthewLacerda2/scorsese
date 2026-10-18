//! Dissolving one shot into the next.

use schemars::JsonSchema;
use scorsese_core::{ClipId, Frames, Project};
use scorsese_render::dissolve;
use scorsese_render::picture::path::OPACITY;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::{load, save};
use crate::tools::{Costs, Reply, Tool};

/// Dissolve one shot into the next.
pub(crate) struct Dissolve;

/// What `dissolve` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the outgoing clip — the shot being left. It fades out over its own
    /// last frames and stays where it is.
    from: Name,
    /// Id of the incoming clip — the shot arriving. It fades in, moves to a
    /// track above, and starts earlier than it did by the length of the
    /// crossover.
    to: Name,
    /// How long the crossover lasts. Default 0.5. Rounded to whole frames on the
    /// project's grid and never to zero — a crossover of no length is a cut, and
    /// asking for one is refused.
    seconds: Option<f64>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("from", "a clip id"), ("to", "a clip id")];
}

impl Tool for Dissolve {
    fn name(&self) -> &'static str {
        "dissolve"
    }

    fn description(&self) -> &'static str {
        "Dissolve one shot into the next, by writing ordinary opacity keyframes \
         on both clips — the same ones you would place by hand, and they stay \
         editable afterwards. Two clips on one track may not overlap and a \
         crossover needs them to, so the incoming clip is MOVED to a track above \
         and pulled back over the outgoing one; the reply says what that \
         rearranged. Each clip carries one opacity track, so a fade already on \
         either is replaced by the dissolve's, and the reply names it. The \
         clips must currently meet at a cut and each must be at \
         least as long as the crossover, or nothing is changed at all."
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
        let (from, to) = (arguments.from.as_str(), arguments.to.as_str());

        let seconds = arguments.seconds.unwrap_or(0.5);
        let duration = Frames::span(seconds, project.timeline_fps.as_f64());
        let faded = [from, to].map(|clip| (clip, opacity_points(&project, clip)));
        let placed = dissolve(&mut project, &ClipId::new(from), &ClipId::new(to), duration)
            .map_err(|error| format!("{error} — nothing was changed"))?;
        save(&project, dir)?;

        let made = if placed.track_created { " (new)" } else { "" };
        Ok(format!(
            "`{from}` dissolves into `{to}` over {} frames — ordinary opacity \
             keyframes, editable and deletable. `{to}` moved to track `{}`{made} \
             and now starts {} frames earlier.{}",
            duration.get(),
            placed.track,
            placed.pulled_back,
            replaced(&faded)
        )
        .into())
    }
}

/// How many opacity points `clip` carried before the dissolve took its
/// opacity over — zero when it was not animating opacity at all.
fn opacity_points(project: &Project, clip: &str) -> usize {
    project.clip(&ClipId::new(clip)).map_or(0, |found| {
        found
            .keyframes
            .iter()
            .filter(|track| track.property.as_str() == OPACITY)
            .map(|track| track.keyframes.len())
            .sum()
    })
}

/// The opacity the dissolve displaced, said because it is gone: a clip carries
/// one opacity track (#1099), so a fade written on it before is replaced.
fn replaced(faded: &[(&str, usize)]) -> String {
    let each: Vec<String> = faded
        .iter()
        .filter(|(_, points)| *points > 0)
        .map(|(clip, points)| format!("`{clip}` ({points} point(s))"))
        .collect();
    if each.is_empty() {
        return String::new();
    }
    format!(
        " It replaced the opacity keyframes already on {}.",
        each.join(" and ")
    )
}
