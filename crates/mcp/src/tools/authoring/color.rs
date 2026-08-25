//! A solid colour: a background, a card, a wash under a title.

use scorsese_core::{Inline, authoring};
use serde_json::Value;

use super::{id_property, maybe, properties, refused, required_color, save};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir};

/// Add a `color` asset.
pub(crate) struct ColorNew;

impl Tool for ColorNew {
    fn name(&self) -> &'static str {
        "color_new"
    }

    fn description(&self) -> &'static str {
        "Add a colour asset: a solid fill for a background, a colour card, or a \
         wash under a title. It fills whatever raster the render is, so there is \
         no size to choose and nothing that ties it to a resolution — and with \
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
        let mut properties = properties(&["color"]);
        properties.insert("asset".to_owned(), id_property("the kind"));
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": ["project", "color"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let color = required_color(arguments, "color", "the colour the card is")?;
        let id = authoring::add_asset(
            &mut project,
            maybe(arguments, "asset").as_deref(),
            Inline::Color(color),
        )
        .map_err(refused)?;
        save(&project, &dir)?;
        Ok(format!(
            "`{id}` — a color asset, {color}. It fills the frame; place_clip puts \
             it on a video track, with a duration."
        )
        .into())
    }
}
