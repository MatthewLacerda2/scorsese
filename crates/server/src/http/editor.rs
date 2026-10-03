//! The web editor's edits (#545): `POST /api/projects/{id}/tools/{name}`.
//!
//! **An edit by hand is a tool call.** A clip dragged along its track is
//! `trim_clip` and onto another lane `clip_move`, the Delete key is
//! `clip_remove`, a file dropped onto a lane is `import` and then `place_clip`,
//! a value typed into the inspector is `clip_set`, a new lane is `track_new`,
//! the preview's frame is `still`, and a selection saved as a template or a
//! template put in at the playhead is `template_save` or `template_insert`
//! (#546) — run by
//! [`Toolbox::edit`](crate::tools::Toolbox::edit), on the same code the
//! assistant and web MCP reach, and recorded in `tool_calls` as client
//! `editor`. So no edit is written twice, once in Rust and again in
//! TypeScript, and a refusal reads the same whoever made the edit.
//!
//! **Only what the editor needs.** The browser's hand-edits are few on
//! purpose (`CLAUDE.md`: the GUI is thin; anything with structure to it is a
//! sentence to the assistant), so this serves an allowlist, [`EDITS`] and
//! [`UNPINNED`], rather than every tool: a page has no business writing a whole
//! document (`project_write`) or spending money (`generate`), and a smaller
//! surface is a smaller thing to hold to per-user rules.
//!
//! **An edit names the revision it was worked out on.** A drag is computed on
//! the timeline the user was looking at, so an edit carries `revision`, and a
//! project that has moved on since — the assistant was working, another tab
//! saved — is `409`: nothing is written, the page reads the project again and
//! the user sees what is there now. The same rule the projects API keeps
//! (#534). Adding a library file, looking at a frame and saving a template
//! change nothing a drag was computed on, and inserting one is worked out on
//! the project as the server finds it — where each track lands is `core`'s
//! rule, not something the user drew — so those take no revision.
//!
//! **A browser session only.** A program holding a token has web MCP, the whole
//! surface; this route is the page's, which is what makes `editor` in the log
//! mean the user's own hands.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use scorsese_mcp::Reply;
use serde::Deserialize;
use serde_json::{Value, json};

use super::AppState;
use super::auth::{Member, Via};
use super::error::ApiError;
use crate::events::Event;
use crate::projects;
use crate::tools::Refusal;

/// The tools that change what a drag is computed on, so each names a revision.
pub const EDITS: [&str; 7] = [
    "track_new",
    "place_clip",
    "trim_clip",
    "clip_set",
    "clip_move",
    "clip_remove",
    "sequence",
];

/// The tools that do not: bringing a library file into the assets table,
/// looking at a frame, saving clips as a template and inserting one.
pub const UNPINNED: [&str; 4] = ["import", "still", "template_save", "template_insert"];

/// `POST /api/projects/{id}/tools/{name}`'s body.
#[derive(Debug, Deserialize)]
pub struct Call {
    /// The tool's arguments, without `project`: the path names it.
    #[serde(default)]
    pub arguments: Value,
    /// The revision the edit was worked out on. Required by [`EDITS`].
    #[serde(default)]
    pub revision: Option<i64>,
}

/// `POST /api/projects/{id}/tools/{name}`: run one of the editor's tools on
/// the caller's project `id`.
///
/// `200 {said, project}`: what the tool answered — its words, and a PNG in
/// base64 when it has a picture — and the project as it is now, revision and
/// document, so the page redraws without asking again (`null` after a
/// `still`). A refused edit is `422` with the tool's own reason, a project
/// that moved on is `409`, a tool the editor does not call is `404`.
pub async fn call(
    State(state): State<AppState>,
    member: Member,
    Path((id, name)): Path<(i64, String)>,
    Json(call): Json<Call>,
) -> Result<Json<Value>, ApiError> {
    if !matches!(member.via, Via::Session(_)) {
        return Err(ApiError::Forbidden(
            "this is the web editor's route and takes a browser session; a program uses web MCP",
        ));
    }
    let edit = EDITS.contains(&name.as_str());
    if !edit && !UNPINNED.contains(&name.as_str()) {
        return Err(ApiError::Refused {
            status: StatusCode::NOT_FOUND,
            message: format!("the editor does not call `{name}` — ask the assistant instead"),
            detail: json!({}),
        });
    }
    let at = match (edit, call.revision) {
        (true, None) => {
            return Err(ApiError::BadRequest(format!(
                "`{name}` is an edit, so it names the revision it was worked out on"
            )));
        }
        (true, at) => at,
        (false, _) => None,
    };
    let mut arguments = match call.arguments {
        Value::Object(fields) => Value::Object(fields),
        Value::Null => json!({}),
        _ => return Err(ApiError::BadRequest("`arguments` is an object".to_owned())),
    };
    arguments["project"] = json!(id);
    // Opened first so another user's project, or none, is the plain `404`
    // every other route gives — before anything is run or recorded.
    let before = projects::open(&state.pool, member.user, id).await?;
    if at.is_some_and(|at| at != before.summary.revision) {
        return Err(moved());
    }
    state
        .limits
        .take(member.user, 1)
        .map_err(|wait| ApiError::Refused {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: format!(
                "too many edits this minute; try again in {} seconds",
                wait.as_secs().max(1)
            ),
            detail: json!({}),
        })?;

    let reply = match state.tools.edit(member.user, &name, &arguments, at).await {
        Ok(reply) => reply,
        Err(Refusal::Moved) => return Err(moved()),
        Err(Refusal::Said(why)) => {
            return Err(ApiError::Refused {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                message: why,
                detail: json!({}),
            });
        }
    };
    let project = if name == "still" {
        None
    } else {
        let now = projects::open(&state.pool, member.user, id).await?;
        if now.summary.revision != before.summary.revision {
            let revision = now.summary.revision;
            state
                .events
                .send(member.user, Event::Project { id, revision });
        }
        Some(now)
    };
    Ok(Json(json!({ "said": said(&reply), "project": project })))
}

/// The refusal for a project that moved on since the edit was worked out.
fn moved() -> ApiError {
    ApiError::Conflict(Refusal::Moved.to_string())
}

/// A reply's parts as JSON: each one's words, and its picture when it has one.
fn said(reply: &Reply) -> Vec<Value> {
    reply
        .parts
        .iter()
        .map(|part| json!({ "text": part.text, "image": part.image }))
        .collect()
}
