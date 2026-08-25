//! A symbol from the set this build ships, named rather than imported.

use scorsese_core::{Icon, Inline, authoring};
use serde_json::Value;

use super::{
    id_property, maybe, number, properties, refused, required_color, required_number, save, words,
};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir};

/// Add an `icon` asset.
pub(crate) struct IconNew;

impl Tool for IconNew {
    fn name(&self) -> &'static str {
        "icon_new"
    }

    fn description(&self) -> &'static str {
        "Add an icon asset: one of the seventeen hundred symbols this build \
         ships, named rather than imported. Call `icons` first to find the name \
         — a name that is not in the catalogue is refused by project_check and \
         by the render, not here. A name is a few bytes, sharp at 4K, and \
         recoloured by editing one string, which is the whole reason not to \
         author a symbol as a PNG somewhere else. Size and colour are both \
         required: a symbol drawn at a size nobody chose, in a colour nobody \
         chose, is a shot that is wrong with nothing to say so. Validated before \
         it is written, and a refusal changes nothing."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        let mut properties = properties(&["size", "color", "stroke_width"]);
        properties.insert(
            "name".to_owned(),
            serde_json::json!({
                "type": "string",
                "description": "Which symbol, by the catalogue's own name for it — \
                                lowercase and hyphenated, `clapperboard` or \
                                `circle-play`. The `icons` tool finds one from a word."
            }),
        );
        properties.insert("asset".to_owned(), id_property("the symbol's name"));
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": ["project", "name", "size", "color"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let icon = Icon::new(
            words(arguments, "name", "which symbol to draw")?,
            required_number(
                arguments,
                "size",
                "how big, as a fraction of the frame's height",
            )?,
            required_color(arguments, "color", "the colour to draw it in")?,
        );
        let icon = match number(arguments, "stroke_width")? {
            Some(width) => icon.weighing(width),
            None => icon,
        };
        let name = icon.name.clone();
        let id = authoring::add_asset(
            &mut project,
            maybe(arguments, "asset").as_deref(),
            Inline::Icon(icon),
        )
        .map_err(refused)?;
        save(&project, &dir)?;
        Ok(format!(
            "`{id}` — an icon asset drawing `{name}`. project_check says whether \
             that name is one this build ships."
        )
        .into())
    }
}
