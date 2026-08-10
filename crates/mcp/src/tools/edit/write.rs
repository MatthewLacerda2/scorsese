//! Replacing the project document wholesale.

use scorsese_core::{Baseline, Project};
use serde_json::Value;

use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

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
         on disk is left exactly as it was. Takes the `fingerprint` project_read \
         reported for the document this edit was made against: if something else \
         has replaced that document since, the write is refused rather than \
         dropping the change, and the answer is to read the project again and \
         redo the edit on what is there now."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
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
                },
                "fingerprint": {
                    "type": "string",
                    "description": "The fingerprint project_read reported for the \
                                    document this edit was made against. It is what \
                                    proves the edit is a change to what is on disk now \
                                    rather than to a version something else has already \
                                    replaced."
                }
            },
            "required": ["project", "document", "fingerprint"]
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
        let mut project = Project::from_json(document)
            .map_err(|problem| format!("refused, nothing written: {problem}"))?;
        project
            .validate()
            .map_err(|problems| format!("refused, nothing written:\n{problems}"))?;

        // The caller's read, carried across the wire. A document that arrived
        // as a string was not read from disk by this process, so this is the
        // only thing that can say which version the edit is a change to. An
        // absent one is deliberately not refused here: the save refuses it,
        // and one refusal worded in one place beats two that can drift.
        if let Some(fingerprint) = arguments.get("fingerprint").and_then(Value::as_str) {
            project.baseline = Baseline::claimed(fingerprint);
        }
        project
            .save(&dir)
            .map_err(|error| format!("refused, nothing written: {error}"))?;
        Ok(format!(
            "written: {} asset(s), {} track(s), {} clip(s)",
            project.assets.len(),
            project.tracks.len(),
            project.clips().count()
        )
        .into())
    }
}
