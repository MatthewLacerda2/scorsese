//! `project_list` and `project_new`: a web user's projects are rows, so
//! finding one and making one are the server's to answer.
//!
//! Locally a project is a directory the caller already knows the path of, and
//! `project_new` makes one. Here a project is named by an id the caller cannot
//! know until it asks, which is what `project_list` is for; and `project_new`
//! stores a new document — `Project::new`, exactly as `scorsese new` makes
//! one — rather than creating a directory.

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
its tracks from track_new.";

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
    let project = Project::new(name, fps);
    let stored = projects::create(&caller.toolbox.pool, caller.user, &project)
        .await
        .map_err(database)?;
    Ok(format!(
        "Created project \"{}\" — its id is {}. Pass project: {} to every other tool.",
        stored.name, stored.id, stored.id
    )
    .into())
}
