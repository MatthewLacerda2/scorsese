//! Dissolving one shot into the next.

use schemars::JsonSchema;
use scorsese_core::ClipId;
use scorsese_render::dissolve;
use serde::Deserialize;
use serde_json::Value;

use super::frames;
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
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
         rearranged. The clips must currently meet at a cut and each must be at \
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
        let duration = frames(seconds, project.timeline_fps.as_f64());
        let placed = dissolve(&mut project, &ClipId::new(from), &ClipId::new(to), duration)
            .map_err(|error| format!("{error} — nothing was changed"))?;
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        let made = if placed.track_created { " (new)" } else { "" };
        Ok(format!(
            "`{from}` dissolves into `{to}` over {} frames — ordinary opacity \
             keyframes, editable and deletable. `{to}` moved to track `{}`{made} \
             and now starts {} frames earlier.",
            duration.get(),
            placed.track,
            placed.pulled_back
        )
        .into())
    }
}
