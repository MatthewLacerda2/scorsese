//! Changing the shape of a project's frame (#1278).

use schemars::JsonSchema;
use scorsese_core::Ratio;
use scorsese_render::reframe;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::{load, save};
use crate::tools::{Costs, Reply, Tool};

/// Change the shape of the project's frame.
pub(crate) struct Reframe;

/// What `project_reframe` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The new shape, across:down — "9:16" for an upright feed, "16:9"
    /// landscape, "1:1" square, "4:5" a portrait post; any ratio within 4:1 of
    /// square.
    aspect: String,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("aspect", "a shape like \"9:16\"")];
}

impl Tool for Reframe {
    fn name(&self) -> &'static str {
        "project_reframe"
    }

    fn description(&self) -> &'static str {
        "Change the shape of the project's frame (its aspect), e.g. from 16:9 to \
         9:16 to deliver a landscape cut upright. Every default follows the \
         shape: render's size, still's, and the frame project_describe and \
         project_check measure against. Never refused. Positions, sizes, text \
         and shapes are fractions of the frame and follow on their own; footage \
         that filled the old frame is set to fill the new one (fit becomes \
         fill, cropping what falls outside). What that cannot settle is listed \
         in project_check's words — a title now cut off its band, a clip now \
         off the frame — for you to fix with clip_set or asset_set; look at it \
         with still afterwards. Use this, not project_write, to change aspect: \
         a bare write leaves footage that filled the old frame in bars."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let to: Ratio = arguments
            .aspect
            .parse()
            .map_err(|problem| format!("aspect: {problem}"))?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let reframed = reframe(&mut project, dir, to);
        if reframed.refit.changed() {
            save(&project, dir)?;
        }
        Ok(reframed.to_string().into())
    }
}
