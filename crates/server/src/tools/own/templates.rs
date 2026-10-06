//! `template_list`, `template_save` and `template_insert`: a user's templates
//! (#546), which are rows, so the server's own tools.
//!
//! What a template *is* and where one lands are `scorsese_core::template`'s:
//! saving is its `extract` over the stored project, inserting its `insert` into
//! one. These tools only find the rows and say what happened. Local folders
//! have no templates yet; when they do, the same two functions serve them.

use std::collections::BTreeSet;

use scorsese_core::template::{self, Description, Inserted};
use scorsese_core::{ClipId, Frames};
use scorsese_mcp::Reply;
use serde_json::{Value, json};

use super::super::surface::project_property;
use super::super::{Caller, database, project_id};
use crate::projects::{self, ProjectError};
use crate::templates::{self, Summary, TemplateError};

/// How a client names the list.
pub(super) const LIST: &str = "template_list";

/// What the list does.
pub(super) const LIST_SAYS: &str = "List your templates — pieces of an edit you saved to reuse: \
an intro, an outro, a running gag, the shape of a video you make every day. Each line is the \
id template_insert takes, the name, how long it runs, how many clips on how many tracks, the \
assets it shows, and — on the line after, when it was saved with one — what it is for. When the user asks for \"my usual intro\" or \"today's video from my \
template\", this is where to find it.";

/// How a client names the saver.
pub(super) const SAVE: &str = "template_save";

/// What the saver does.
pub(super) const SAVE_SAYS: &str = "Save some of a project's clips as a template in your \
account, to insert into any of your projects later. The template keeps the clips exactly as \
they are — keyframes, speed, fit, everything — with their tracks and the assets they show, and \
starts where its earliest clip starts. Files are not copied: the template names your library \
files, so a generated intro is paid for once and reused for free. Inserting it later copies \
the clips, so changing or replacing a template never changes a video it was already used in. \
An arrow must be saved with the clip it follows. Say what the template is for in \
description, so it can be told from the others later.";

/// How a client names the inserter.
pub(super) const INSERT: &str = "template_insert";

/// What the inserter does.
pub(super) const INSERT_SAYS: &str = "Copy one of your templates into a project, its first \
clip starting at at_seconds. Its tracks land on the project's by position among tracks of the \
same kind — the template's first video track on the project's first video track, its second \
on the second, audio alike — and where something is already in the way, that track and every \
one above it of that kind go onto new tracks on top, so the template's own layering is kept. \
Nothing already in the project moves. Clip and asset ids are kept where free and suffixed \
(-2) where not; files the project already has are shared, not added twice. The reply names \
every clip placed and the track it went on, which is what clip_move and clip_set take next.";

/// `template_save`'s arguments.
pub(super) fn save_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project": project_property(),
            "clips": {
                "type": "array",
                "items": { "type": "string" },
                "minItems": 1,
                "description": "The ids of the clips to save, as project_read shows them."
            },
            "name": {
                "type": "string",
                "description": "What to call the template — how you and the user will ask \
                                for it. Unique among your templates, ignoring case."
            },
            "description": {
                "type": "string",
                "description": "What the template is for, in a sentence or two — \"the intro \
                                every daily video opens with\". template_list shows it, so \
                                you and the user can pick the right one later. At most \
                                2000 characters. Optional; when replacing, leaving it out \
                                keeps the old one."
            },
            "replace": {
                "type": "boolean",
                "description": "Overwrite the template that already has this name, instead \
                                of being refused. Default false."
            }
        },
        "required": ["project", "clips", "name"]
    })
}

/// `template_insert`'s arguments.
pub(super) fn insert_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project": project_property(),
            "template": {
                "type": "integer",
                "description": "The template to insert, by the id template_list shows."
            },
            "at_seconds": {
                "type": "number",
                "description": "Where on the timeline the template's first clip starts, in \
                                seconds — 0 for an intro, the end of the cut for an outro. \
                                Rounded to a whole frame."
            }
        },
        "required": ["project", "template", "at_seconds"]
    })
}

/// A template as a line, and what it is for on the next when it says.
fn line(template: &Summary) -> String {
    let mut line = format!(
        "{} — {} ({:.1}s, {} clips on {} tracks: {})",
        template.id,
        template.name,
        template.seconds,
        template.clips,
        template.tracks,
        template.assets.join(", ")
    );
    if let Some(description) = &template.description {
        line.push_str(&format!("\n    for: {description}"));
    }
    line
}

/// The caller's templates.
pub(super) async fn list(caller: &Caller<'_>) -> Result<Reply, String> {
    let listed = templates::list(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    if listed.is_empty() {
        return Ok(
            "You have no templates yet. template_save makes one from clips in a project.".into(),
        );
    }
    Ok(listed
        .iter()
        .map(line)
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

/// Clips of a project, saved as a template.
pub(super) async fn save(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let project = project_id(arguments)?;
    let clips: BTreeSet<ClipId> = arguments
        .get("clips")
        .and_then(Value::as_array)
        .map(|clips| {
            clips
                .iter()
                .filter_map(Value::as_str)
                .map(ClipId::new)
                .collect()
        })
        .unwrap_or_default();
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or("`name` is required: what to call the template")?;
    let description = arguments
        .get("description")
        .and_then(Value::as_str)
        .map(Description::new)
        .transpose()
        .map_err(|error| format!("{error} — nothing was saved"))?
        .flatten();
    let replace = arguments
        .get("replace")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let pool = &caller.toolbox.pool;
    let stored = projects::open(pool, caller.user, project)
        .await
        .map_err(opened)?;
    let fragment = template::extract(&stored.document, &clips, name)
        .map_err(|error| format!("{error} — nothing was saved"))?;
    let saved = templates::save(pool, caller.user, &fragment, description.as_ref(), replace)
        .await
        .map_err(said)?;
    Ok(format!(
        "Saved template {}. template_insert puts it in any of your projects.",
        line(&saved)
    )
    .into())
}

/// A template, copied into a project.
pub(super) async fn insert(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let project = project_id(arguments)?;
    let id = arguments
        .get("template")
        .and_then(Value::as_i64)
        .ok_or("`template` is required: the id template_list shows")?;
    let seconds = arguments
        .get("at_seconds")
        .and_then(Value::as_f64)
        .filter(|at| *at >= 0.0)
        .ok_or("`at_seconds` is required: where the template starts, from 0")?;
    let pool = &caller.toolbox.pool;
    let template = templates::open(pool, caller.user, id).await.map_err(said)?;
    // Another writer between the read and the save re-runs the insertion on
    // what is there now: where a template lands is worked out on the project
    // as it is, not on anything the caller saw.
    for _ in 0..3 {
        let stored = projects::open(pool, caller.user, project)
            .await
            .map_err(opened)?;
        let mut document = stored.document;
        let at = document.timeline_fps.frames(seconds);
        let done = template::insert(&mut document, &template.document, at)
            .map_err(|error| format!("{error} — nothing was inserted"))?;
        match projects::save(
            pool,
            caller.user,
            project,
            stored.summary.revision,
            &document,
        )
        .await
        {
            Ok(_) => {
                return Ok(report(&template.summary.name, &document.timeline_fps, &done).into());
            }
            Err(ProjectError::Conflict { .. }) => {}
            Err(refused @ ProjectError::UnknownFiles { .. }) => return Err(refused.to_string()),
            Err(error) => return Err(database(error)),
        }
    }
    Err("the project kept changing while this ran, so nothing was inserted; call again".to_owned())
}

/// What an insertion did, in words.
fn report(name: &str, fps: &scorsese_core::Fps, done: &Inserted) -> String {
    let seconds = |at: Frames| fps.seconds(at);
    let placed: Vec<String> = done
        .clips
        .iter()
        .map(|(clip, track)| format!("{clip} on {track}"))
        .collect();
    let mut said = format!(
        "Inserted \u{201c}{name}\u{201d} from {:.2}s to {:.2}s: {}.",
        seconds(done.start),
        seconds(done.end),
        placed.join(", ")
    );
    let listed = |ids: &[String]| ids.join(", ");
    if !done.new_tracks.is_empty() {
        let tracks: Vec<String> = done.new_tracks.iter().map(ToString::to_string).collect();
        said.push_str(&format!(" New tracks, on top: {}.", listed(&tracks)));
    }
    if !done.added.is_empty() {
        let assets: Vec<String> = done.added.iter().map(ToString::to_string).collect();
        said.push_str(&format!(" Assets added: {}.", listed(&assets)));
    }
    if !done.reused.is_empty() {
        let assets: Vec<String> = done.reused.iter().map(ToString::to_string).collect();
        said.push_str(&format!(" Already in the project: {}.", listed(&assets)));
    }
    said
}

/// A project that could not be opened, said to the caller.
fn opened(error: ProjectError) -> String {
    match error {
        ProjectError::NotFound => "there is no such project of yours".to_owned(),
        other => database(other),
    }
}

/// A template failure, said to the caller.
fn said(error: TemplateError) -> String {
    match error {
        TemplateError::Database(_) | TemplateError::Serialize(_) => database(error),
        refused => refused.to_string(),
    }
}
