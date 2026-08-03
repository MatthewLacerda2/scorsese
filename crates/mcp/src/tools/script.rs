//! The script the project carries: reading it, and writing it.
//!
//! Every other thing an assistant needs is inside `project.json`, which
//! `project_read` and `project_write` already hand over whole — notes
//! included, since a note is a field of the element it is about. The script is
//! the one part of a project's own reasoning that is **not** in that document:
//! it is a file beside it, because such a document is long enough that inlining
//! it would bury the timeline under prose. So it needs a pair of tools of its
//! own, and without them an assistant can see that a project has a script and
//! has no way to read a word of it.
//!
//! Which is the failure worth naming. A cold start on someone else's project
//! is a guess unless the thing that says what the film has to be — and, as
//! often as not, what must never be claimed on camera — can actually be read
//! before the first cut is moved.

use std::path::Path;

use scorsese_core::{Project, ProjectPath};
use serde_json::Value;

use super::inspect::load;
use super::{Costs, Reply, Tool, project_dir, project_only_schema, project_property};

/// By convention, and only that: the tool never reads the extension.
const DEFAULT_SCRIPT: &str = "script.md";

/// Read the project's script.
pub(super) struct Read;

impl Tool for Read {
    fn name(&self) -> &'static str {
        "script_read"
    }

    fn description(&self) -> &'static str {
        "Read the document this edit is being cut from — the brief, the outline, \
         whatever the project's `script` field points at. Read it before changing \
         anything: it is where the reasons live that the timeline cannot show you, \
         including what the film must not claim. Not parsed, not structured — \
         prose, exactly as written."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        project_only_schema()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let project = load(&dir)?;
        let script = project
            .script
            .as_ref()
            .ok_or_else(|| "this project has no script; script_write starts one".to_owned())?;
        let file = resolve(script, &dir)?;
        // A script the document names and the disk does not have is worth
        // saying plainly rather than as a file-not-found: the project has lost
        // something that cannot be reconstructed by looking at the film.
        std::fs::read_to_string(&file)
            .map(Reply::from)
            .map_err(|error| format!("the project's script `{script}` could not be read: {error}"))
    }
}

/// Replace the project's script.
pub(super) struct Write;

impl Tool for Write {
    fn name(&self) -> &'static str {
        "script_write"
    }

    fn description(&self) -> &'static str {
        "Write the project's script — the document the edit is cut from. Replaces \
         it whole, so read it first unless you are starting one. If the project \
         has no script yet, this creates the file and points the project at it. \
         Nothing here is ever rendered: a script is read, never shown. Use a note \
         on an asset, track or clip for a reason about one thing; use this for what \
         the whole film has to be."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "text": {
                    "type": "string",
                    "description": "The complete script to write. Not a patch — whatever \
                                    is here replaces the file. Prose in any form; markdown \
                                    by convention, and nothing parses it."
                },
                "path": {
                    "type": "string",
                    "description": "Where to keep it, relative to the project root. Only \
                                    used when the project has no script yet; defaults to \
                                    script.md at the root."
                }
            },
            "required": ["project", "text"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let text = arguments
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| "`text` is required: the script to write".to_owned())?;

        let script = script_path(&project, arguments);
        let file = resolve(&script, &dir)?;
        // A script may be kept anywhere inside the project, and refusing to
        // start one at `notes/brief.md` because `notes/` is not there yet would
        // be refusing over something the tool can simply do.
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("making {}: {error}", parent.display()))?;
        }
        scorsese_core::write::atomically(&file, text)
            .map_err(|error| format!("writing {}: {error}", file.display()))?;

        // The field is set only after the bytes have landed. A document
        // pointing at a script that failed to write would be a project claiming
        // something it does not have, which is the one state the missing-script
        // warning exists to report honestly.
        if project.script.as_ref() != Some(&script) {
            project.script = Some(script.clone());
            project
                .save(&dir)
                .map_err(|error| format!("the script was written, but: {error}"))?;
        }
        Ok(format!("written: {script} ({} bytes)", text.len()).into())
    }
}

/// Where the script goes: where the project already keeps it, what the caller
/// asked for, or the conventional name.
///
/// An existing `script` wins over a `path` argument. Moving the script is a
/// different operation from writing it, and doing it as a side effect of a
/// write would leave the old file behind with nothing pointing at it.
fn script_path(project: &Project, arguments: &Value) -> ProjectPath {
    project.script.clone().unwrap_or_else(|| {
        let asked = arguments
            .get("path")
            .and_then(Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .unwrap_or(DEFAULT_SCRIPT);
        ProjectPath::new(asked)
    })
}

/// The real path of a project-relative one, checked before it is opened.
///
/// The same rule the renderer and the synthesiser hold: a path argument is
/// never a way to read or write outside the project.
fn resolve(script: &ProjectPath, dir: &Path) -> Result<std::path::PathBuf, String> {
    script
        .check()
        .map_err(|problem| format!("script path `{script}` {problem}"))?;
    Ok(script.resolve(dir))
}
