//! A caption, a title, a lower third: the one asset an agent writes most.

use scorsese_core::{Inline, authoring};
use serde_json::Value;

use super::{id_property, properties, refused, save, style, wanted, words};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir};

/// Add a `text` asset.
pub(crate) struct TextNew;

impl Tool for TextNew {
    fn name(&self) -> &'static str {
        "text_new"
    }

    fn description(&self) -> &'static str {
        "Add a text asset — a caption, a title, a lower third: what it says, and \
         the look it is set in. This is the single most common thing there is to \
         author in a cut, and the alternative is sending the whole project.json \
         back to change one line. The string lives in the document, so there is \
         no file to import, hash or probe, and nothing to generate. Sizes are \
         fractions of the frame rather than pixels, so one number reads the same \
         at every render resolution. Say nothing about the look and it is a \
         white, centred, sans title. Validated before it is written — a document \
         that would not load is refused with the reason and the project is left \
         exactly as it was. Then place_clip puts it on a video track, and it has \
         no length of its own, so that call needs a duration."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        let mut properties = properties(&[
            "font",
            "weight",
            "italic",
            "size",
            "color",
            "align",
            "line_height",
            "max_width",
        ]);
        properties.insert(
            "text".to_owned(),
            serde_json::json!({
                "type": "string",
                "description": "What it says. Newlines are kept, and anything longer \
                                than `max_width` wraps on its own."
            }),
        );
        properties.insert("asset".to_owned(), id_property("its opening words"));
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": ["project", "text"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let content = Inline::Text {
            text: words(arguments, "text", "what the caption says")?,
            style: style(arguments)?,
        };
        let id = authoring::add_asset(&mut project, wanted(arguments, "asset").as_deref(), content)
            .map_err(refused)?;
        save(&project, &dir)?;
        Ok(format!(
            "`{id}` — a text asset. place_clip puts it on a video track, with a \
             duration: a title has no length of its own."
        )
        .into())
    }
}
