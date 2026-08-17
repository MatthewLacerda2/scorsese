//! Tools that only look: read the document, describe the cut, report faults.
//!
//! None of these change anything, and none of them cost anything to run.

use scorsese_core::{Frames, HashCheck, Project, asset_status, fingerprint_of};
use scorsese_render::{
    Checkup, Commentary, Description, FrameRange, Layout, Note, Plan, Resolution, unknown_in,
};
use serde_json::Value;

use super::{Costs, Part, Reply, Tool, project_dir, project_only_schema};

/// The frame a layout question is answered against when nobody names one.
///
/// The delivery raster rather than the smaller one `still` composites at:
/// nothing is drawn here, so there is no reply size to save, and the shape a
/// caller is reasoning about is the one they will deliver.
const DEFAULT_RASTER: &str = "1920x1080";

/// The project document itself.
pub(super) struct Read;

impl Tool for Read {
    fn name(&self) -> &'static str {
        "project_read"
    }

    fn description(&self) -> &'static str {
        "Read a project's project.json exactly as it is on disk. The whole edit \
         is in this document — assets, tracks, clips, keyframes — so this is the \
         starting point for any change. Pair with project_write to edit it, and \
         hand it the `fingerprint` this reports: it says which document the edit \
         was made against, so a write cannot silently land on a change somebody \
         else made in the meantime. The \
         format is documented in docs/project-format.md."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        project_only_schema()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        // Read rather than load-and-serialise, so what comes back is the file
        // as written — including whatever a hand edit left in it. A document
        // that will not validate is exactly when reading it matters most.
        let document = std::fs::read_to_string(dir.join(scorsese_core::PROJECT_FILE_NAME))
            .map_err(|error| format!("reading the project: {error}"))?;
        // Its own part, after the document rather than woven into it: the
        // first block is the file and nothing else, so a client that parses
        // what it was handed still gets a project.
        let fingerprint = fingerprint_of(document.as_bytes());
        Ok(vec![
            Part::words(document),
            Part::words(format!(
                "fingerprint: {fingerprint}\nPass this to project_write with the edit. It \
                 says which document the edit was made against, and a write built on one \
                 something else has since replaced is refused rather than dropping that \
                 change."
            )),
        ]
        .into())
    }
}

/// What the timeline contains.
pub(super) struct Describe;

impl Tool for Describe {
    fn name(&self) -> &'static str {
        "project_describe"
    }

    fn description(&self) -> &'static str {
        "Say what the cut contains, shot by shot and sound by sound. That is \
         what is on screen when, on which track, at what fit, with what \
         animated, and what is audible under it — and every note left on an \
         asset, track or clip saying why it is that way. \
         Sequences the timeline exactly as a render would but produces no file, \
         so it is the cheapest way to check an edit is right. No ffmpeg, no cost. \
         Pass `at` to also get *where* each clip's content lands at that instant: \
         the rectangle a title's wrapped block, a shape's box or a fitted picture \
         covers, in fractions of the frame. That is the number you would \
         otherwise guess at, render, look at and adjust — and it is the \
         compositor's own rather than a second calculation of it, so a panel \
         sized from it fits the text it sits behind."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": super::project_property(),
                "at": {
                    "type": ["string", "array"],
                    "items": { "type": "string" },
                    "description": "Also say where every clip's content lands at this \
                                    instant: a time like 9.1s, or a timeline frame number \
                                    like 285. A bare decimal is refused — say which unit \
                                    you mean. Give a list, e.g. [\"0s\", \"9.1s\", \"400\"], \
                                    to ask about several: a block that fits at one instant \
                                    may not at another, where a longer caption has taken \
                                    its place. Every rectangle comes back in fractions of \
                                    the frame — the unit transform.position, a text size \
                                    and a shape's width are all written in — so a number \
                                    read here can be written straight back into the \
                                    document. Clips with no rectangle are named too, with \
                                    the reason: not on screen at that instant, sound only, \
                                    or on screen and immeasurable."
                },
                "resolution": {
                    "type": "string",
                    "description": "The frame `at` is measured against, e.g. 1920x1080, \
                                    which is the default. The answer is in fractions, but \
                                    the frame's shape still decides it: a `fit` picture \
                                    letterboxes against this aspect and a title wraps \
                                    against this width. Nothing is composited either way."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let project = load(&dir)?;
        let instants = asked_about(arguments, project.timeline_fps)?;
        let raster = raster(arguments)?;
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
        // Each instant on its own and in the order it was asked about: a list
        // of instants is a list of questions, and the answers have to line up
        // with them.
        for at in instants {
            let layout = Layout::at(&project, &dir, raster, at)
                .map_err(|error| format!("frame {}: {error}", at.get()))?;
            out.push_str(&format!("\n{layout}\n"));
        }
        Ok(out.into())
    }
}

/// The raster a layout question is measured against — the delivery size unless
/// the caller named another.
///
/// A default at all, rather than the project's own anything, because a project
/// does not carry a raster: resolution is chosen per render, and a question
/// about where a title sits has to be asked against *some* frame.
fn raster(arguments: &Value) -> Result<Resolution, String> {
    arguments
        .get("resolution")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_RASTER)
        .parse()
        .map_err(|problem| format!("resolution: {problem}"))
}

/// Which instants the caller wants a layout for, if any. No `at` is the
/// question this tool always answered, and it stays a whole answer on its own.
fn asked_about(arguments: &Value, fps: scorsese_core::Fps) -> Result<Vec<Frames>, String> {
    match arguments.get("at") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(_) => super::still::instants(arguments, fps),
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

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": super::project_property(),
                "verify": {
                    "type": "boolean",
                    "description": "Re-hash every file to catch media edited behind \
                                    the project's back since it was imported. Off by \
                                    default, because this is the tool to call after \
                                    every edit and re-reading a whole pool is slow: \
                                    whether a file is *there* is checked either way \
                                    and costs nothing."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let file = dir.join(scorsese_core::PROJECT_FILE_NAME);
        let json = std::fs::read_to_string(&file)
            .map_err(|error| format!("opening {}: {error}", file.display()))?;
        // Parsed rather than loaded, because `Project::load` validates and
        // returns nothing to warn about when it refuses — and a document that
        // does not validate is exactly the one there is most to say about. A
        // document that will not *parse* is as far as this can get, and saying
        // so is the answer rather than an error: being asked what is wrong and
        // finding something is this tool working.
        let project = match Project::from_json(&json) {
            Ok(project) => project,
            Err(problem) => return Ok(problem.to_string().into()),
        };

        let verify = arguments.get("verify").and_then(Value::as_bool) == Some(true);
        let hashes = if verify {
            HashCheck::Verify
        } else {
            HashCheck::Skip
        };
        // The same assembly `scorsese check` prints, so one project cannot get
        // two different answers depending on which surface asked.
        let checkup = Checkup::of(&project, &dir, hashes);
        let mut out = String::new();
        for line in checkup.lines() {
            out.push_str(&format!("{line}\n"));
        }
        out.push_str(checkup.summary());
        out.push('\n');
        if !verify {
            out.push_str("(hashes not checked — pass verify: true to re-hash every file)\n");
        }
        out.push_str(checkup.verdict().says());
        Ok(out.into())
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

    fn costs(&self) -> Costs {
        Costs::Nothing
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
