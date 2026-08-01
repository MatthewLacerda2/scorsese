//! Replacing the project document wholesale.

use scorsese_core::Project;
use serde_json::Value;

use crate::tools::{Reply, Tool, project_dir, project_property};

/// Replace the project document.
pub(crate) struct Write;

impl Tool for Write {
    fn name(&self) -> &'static str {
        "project_write"
    }

    fn description(&self) -> &'static str {
        "Replace a project's project.json with the document given. The whole \
         edit is this file, so this is how any change is made: read it, change \
         it, write it back. **Validated before it is written** — a document \
         that would not load is refused with every problem listed, and the file \
         on disk is left exactly as it was."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "document": {
                    "type": "string",
                    "description": "The complete project.json to write, as text. \
                                    Not a patch — whatever is here replaces the file."
                }
            },
            "required": ["project", "document"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let document = arguments
            .get("document")
            .and_then(Value::as_str)
            .ok_or_else(|| "`document` is required: the project.json to write".to_owned())?;

        // Parsed *and* validated before anything is written. An editor may save
        // work that is temporarily incoherent — a person with the file open —
        // but a tool writing a whole document has no such excuse, and a broken
        // project.json is the one thing that makes every other tool useless.
        let project = Project::from_json(document)
            .map_err(|problem| format!("refused, nothing written: {problem}"))?;
        project
            .validate()
            .map_err(|problems| format!("refused, nothing written:\n{problems}"))?;

        project
            .save(&dir)
            .map_err(|error| format!("writing the project: {error}"))?;
        Ok(format!(
            "written: {} asset(s), {} track(s), {} clip(s)",
            project.assets.len(),
            project.tracks.len(),
            project.clips().count()
        )
        .into())
    }
}
