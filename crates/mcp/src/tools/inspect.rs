//! Tools that only look: read the document, describe the cut, report faults.
//!
//! None of these change anything, and none of them cost anything to run.

use scorsese_core::{HashCheck, Project, asset_status};
use scorsese_render::{Commentary, Description, FrameRange, Note, Plan, unknown_in};
use serde_json::Value;

use super::{Reply, Tool, project_dir, project_only_schema};

/// The project document itself.
pub(super) struct Read;

impl Tool for Read {
    fn name(&self) -> &'static str {
        "project_read"
    }

    fn description(&self) -> &'static str {
        "Read a project's project.json exactly as it is on disk. The whole edit \
         is in this document — assets, tracks, clips, keyframes — so this is the \
         starting point for any change. Pair with project_write to edit it. The \
         format is documented in docs/project-format.md."
    }

    fn schema(&self) -> Value {
        project_only_schema()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        // Read rather than load-and-serialise, so what comes back is the file
        // as written — including whatever a hand edit left in it. A document
        // that will not validate is exactly when reading it matters most.
        std::fs::read_to_string(dir.join(scorsese_core::PROJECT_FILE_NAME))
            .map(Into::into)
            .map_err(|error| format!("reading the project: {error}"))
    }
}

/// What the timeline contains.
pub(super) struct Describe;

impl Tool for Describe {
    fn name(&self) -> &'static str {
        "project_describe"
    }

    fn description(&self) -> &'static str {
        "Say what the cut contains: what is on screen when, on which track, at \
         what fit, with what animated, and what is audible under it — and every \
         note left on an asset, track or clip saying why it is that way. \
         Sequences the timeline exactly as a render would but produces no file, \
         so it is the cheapest way to check an edit is right. No ffmpeg, no cost."
    }

    fn schema(&self) -> Value {
        project_only_schema()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let project = load(&dir)?;
        let plan = Plan::build(&project, project.timeline_fps, FrameRange::ALL)
            .map_err(|error| format!("sequencing the timeline: {error}"))?;

        // Ahead of the cut, not after it: a script is meant to be read before
        // the edit is touched, and a note is the context the shot only makes
        // sense in.
        let commentary = Commentary::of(&project);
        let mut out = String::new();
        if !commentary.is_empty() {
            out.push_str(&format!("{commentary}\n\n"));
        }
        out.push_str(&format!("{} — {}\n", project.name, Description::of(&plan)));
        for note in plan.notes() {
            out.push_str(&format!("  note: {note}\n"));
        }
        for unknown in unknown_in(&project) {
            out.push_str(&format!("  note: {}\n", Note::from(unknown)));
        }
        Ok(out.into())
    }
}

/// Everything wrong with the project.
pub(super) struct Check;

impl Tool for Check {
    fn name(&self) -> &'static str {
        "project_check"
    }

    fn description(&self) -> &'static str {
        "Report everything wrong or questionable about a project — the document \
         and the media it references — without rendering. Returns every problem \
         at once rather than stopping at the first, so one call is the whole \
         repair job. Call this after any edit."
    }

    fn schema(&self) -> Value {
        project_only_schema()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        match Project::load(&dir) {
            Ok(_) => Ok("no problems with the document".into()),
            // A failure here is the answer, not an error: being asked what is
            // wrong and finding something is this tool working.
            Err(problems) => Ok(problems.to_string().into()),
        }
    }
}

/// What is in the media pool.
pub(super) struct Assets;

impl Tool for Assets {
    fn name(&self) -> &'static str {
        "project_assets"
    }

    fn description(&self) -> &'static str {
        "List the media pool: every asset, its kind, what state it is in, and \
         how many clips use it. Says which generated assets are still sketches \
         nobody has realised, and which files the document points at and cannot \
         find."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": super::project_property(),
                "verify": {
                    "type": "boolean",
                    "description": "Re-hash every file to catch media edited behind \
                                    the project's back. Slow on a large pool; the \
                                    default only checks that files are present."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let project = load(&dir)?;
        let check = if arguments.get("verify").and_then(Value::as_bool) == Some(true) {
            HashCheck::Verify
        } else {
            HashCheck::Skip
        };
        let rows = asset_status(&project, &dir, check);
        if rows.is_empty() {
            return Ok("the pool is empty".into());
        }
        Ok(rows
            .iter()
            .map(|row| {
                format!(
                    "{}\t{:?}\t{:?}\t{} clip(s)",
                    row.id, row.kind, row.health, row.clip_count
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into())
    }
}

/// Loads a project, with the failure worded for a client rather than a shell.
pub(crate) fn load(dir: &std::path::Path) -> Result<Project, String> {
    Project::load(dir).map_err(|error| format!("opening {}: {error}", dir.display()))
}
