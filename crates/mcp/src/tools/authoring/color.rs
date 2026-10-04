//! A solid colour: a background, a card, a wash under a title.

use schemars::JsonSchema;
use scorsese_core::{Inline, authoring};
use serde::Deserialize;
use serde_json::Value;

use super::fill::{self, Paint};
use super::{id_described, maybe, refused, save};
use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Add a `color` asset.
pub(crate) struct ColorNew;

/// What the colour is, when it is missing.
const WHAT: &str = "the colour the card is";

/// What `color_new` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    #[schemars(description = fill::described(
        "The colour, as `#rrggbb` — or `#rrggbbaa` for a scrim the shot underneath shows \
         through."
    ))]
    color: Paint,
    #[schemars(description = id_described("the kind"))]
    asset: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("color", WHAT)];
}

impl Tool for ColorNew {
    fn name(&self) -> &'static str {
        "color_new"
    }

    fn description(&self) -> &'static str {
        "Add a colour asset: a solid colour or a gradient for a background, a \
         colour card, or a wash under a title. A radial gradient lighter behind \
         the subject is the usual motion-graphics backdrop. It fills whatever \
         raster the render is, so there is no size to choose and nothing that \
         ties it to a resolution — and with \
         an alpha it is a scrim that the shot underneath shows through. The \
         colour is required and has no default on purpose: a card nobody chose \
         the colour of would render as some colour, and a film that opens on the \
         wrong shade fails silently. Validated before it is written, and a \
         refusal leaves the project exactly as it was."
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
        // A blank or null colour is somebody not choosing one, which is the
        // one thing this kind refuses — said the way a missing one is.
        let color = fill::fill(Some(&arguments.color), "color")?
            .ok_or_else(|| format!("`color` is required: {WHAT}"))?;
        let said = color.to_string();
        let id = authoring::add_asset(
            &mut project,
            maybe(arguments.asset.as_deref()).as_deref(),
            Inline::Color(color),
        )
        .map_err(refused)?;
        save(&project, dir)?;
        Ok(format!(
            "`{id}` — a color asset, {said}. It fills the frame; place_clip puts \
             it on a video track, with a duration."
        )
        .into())
    }
}
