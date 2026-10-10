//! `project_list` and `project_new`: a web user's projects are rows, so
//! finding one and making one are the server's to answer.
//!
//! Locally a project is a directory the caller already knows the path of, and
//! `project_new` makes one. Here a project is named by an id the caller cannot
//! know until it asks, which is what `project_list` is for; and `project_new`
//! stores a new document — `Project::new`, exactly as `scorsese new` makes
//! one — rather than creating a directory.

use scorsese_core::style::{Platform, Start};
use scorsese_core::{Fps, Project};
use scorsese_mcp::Reply;
use serde_json::{Value, json};

use super::super::{Caller, database};
use crate::projects;

/// How a client names the list.
pub(super) const LIST: &str = "project_list";

/// What the list does.
pub(super) const LIST_SAYS: &str = "List your projects, with the id every other tool takes as \
`project` — newest edit first. Each line is the id, the name, and the revision the project is \
at. Call this first: on the hosted server a project is named by its id, not by a path.";

/// How a client names the maker.
pub(super) const NEW: &str = "project_new";

/// What the maker does.
pub(super) const NEW_SAYS: &str = "Create an empty project in your account and answer with its \
id, which every other tool takes as `project`. The project starts with no assets and no tracks, \
on a 30 fps grid unless fps says otherwise; its files come from your library with import, and \
its tracks from track_new. Give platform and/or style when the person has said where the video \
is going or what kind it is: the project then starts with a brief in script.md (script_read), \
and its renders default to the platform's size. Either way, before editing anything, propose \
the script to the person and agree it: scene by scene, each with its narration, what is on \
screen, and the music or sound.";

/// `project_new`'s arguments.
pub(super) fn new_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "What to call the project — what project_list and the web app \
                                show it as."
            },
            "fps": {
                "type": ["integer", "string"],
                "description": "The timeline's frame rate: a whole number like 30, or a \
                                ratio written \"30000/1001\". Default 30. Every start and \
                                duration is counted in these frames."
            },
            "platform": {
                "type": "string",
                "enum": Platform::ALL.map(Platform::id),
                "description": "The placement the video is made for. Written into the brief \
                                with the render size it means, which the project's renders \
                                then default to."
            },
            "style": {
                "type": "string",
                "description": "The kind of video, by id: kinetic_type, whiteboard, \
                                narrated_captions… An unknown id, or one not made for \
                                platform, is refused with the ids that fit. Its prompt is \
                                written into the brief."
            }
        },
        "required": ["name"]
    })
}

/// The caller's projects.
pub(super) async fn list(caller: &Caller<'_>) -> Result<Reply, String> {
    let projects = projects::list(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    if projects.is_empty() {
        return Ok("You have no projects yet. project_new makes one.".into());
    }
    let lines: Vec<String> = projects
        .iter()
        .map(|project| {
            format!(
                "{} — {} (revision {})",
                project.id, project.name, project.revision
            )
        })
        .collect();
    Ok(lines.join("\n").into())
}

/// A new project.
pub(super) async fn new(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or("`name` is required: what to call the project")?;
    let fps = match arguments.get("fps") {
        None | Some(Value::Null) => Fps::default(),
        Some(given) => given
            .as_u64()
            .map(|whole| whole.to_string())
            .or_else(|| given.as_str().map(str::to_owned))
            .and_then(|text| text.parse::<Fps>().ok())
            .ok_or_else(|| {
                format!("fps: {given} is not a frame rate — a whole number like 30, or 30000/1001")
            })?,
    };
    let start = start(
        arguments.get("platform").and_then(Value::as_str),
        arguments.get("style").and_then(Value::as_str),
    )?;
    let project = Project::new(name, fps);
    let stored = projects::begin(&caller.toolbox.pool, caller.user, project, &start)
        .await
        .map_err(database)?;
    let id = stored.summary.id;
    let mut reply = format!(
        "Created project \"{}\" — its id is {id}. Pass project: {id} to every other tool.",
        stored.summary.name
    );
    if let Some(script) = &stored.document.script {
        reply.push_str(&format!(
            "\n{script}: the brief. Read it, then propose the script, scene by scene."
        ));
    }
    Ok(reply.into())
}

/// The platform and style asked for, checked before anything is written, in
/// the words the local `project_new` refuses them in.
fn start(platform: Option<&str>, style: Option<&str>) -> Result<Start, String> {
    let platform = platform
        .map(str::parse::<Platform>)
        .transpose()
        .map_err(|problem| format!("platform: {problem}"))?;
    Start::new(platform, style).map_err(|problem| format!("style: {problem}"))
}
