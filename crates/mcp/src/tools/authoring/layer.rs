//! Moving a video lane under or over another (#1117).

use schemars::JsonSchema;
use scorsese_core::{TrackId, authoring};
use serde::Deserialize;
use serde_json::Value;

use super::track::{layer, sits};
use super::{refused, save};
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Move a video track in the stack of layers.
pub(crate) struct TrackMove;

/// What `track_move` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The id of the video track to move, as project_read names it.
    track: Name,
    /// The id of the video track to put it directly UNDER. Give this or
    /// `over`.
    under: Option<String>,
    /// The id of the video track to put it directly OVER. Give this or
    /// `under`.
    over: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("track", "the id of the video track to move")];
}

impl Tool for TrackMove {
    fn name(&self) -> &'static str {
        "track_move"
    }

    fn description(&self) -> &'static str {
        "Move a video track up or down the stack of layers: directly under or \
         directly over another video track, its clips going with it. Video \
         tracks composite in order, each drawn over the ones below it, so this \
         is \"put the logo under the captions\". Nothing else changes — no clip \
         moves in time. Audio tracks have no layer (everything audible is \
         summed), and a lane inside a group moves only among that group's lanes."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let to = layer(arguments.under.as_deref(), arguments.over.as_deref())?
            .ok_or("`under` or `over` is required: the id of the video track to move it beside")?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let track = TrackId::new(arguments.track.as_str());
        authoring::move_track(&mut project, &track, &to).map_err(refused)?;
        save(&project, dir)?;
        Ok(format!("`{track}` is now {}.", sits(Some(&to))).into())
    }
}
