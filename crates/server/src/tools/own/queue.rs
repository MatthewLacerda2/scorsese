//! `render` and `jobs`: work the queue does, asked for and asked after.
//!
//! Locally `render` writes a file wherever `out` says and returns when it is
//! done. On the server a render is a job (#541) — the machine is shared, and
//! two at once is what it carries — and its file is kept in the render cache
//! for download. So `render` asks for one exactly as `POST
//! /api/projects/{id}/renders` does ([`crate::renders::request`]), and `jobs`
//! is how a client learns it finished, as it is for a generation.

use scorsese_mcp::Reply;
use serde_json::{Value, json};

use super::super::surface::project_property;
use super::super::{Caller, database, project_id};
use crate::jobs::{JobView, State, store as jobs};
use crate::projects::ProjectError;
use crate::renders::request::{AskError, Asked, ask};
use crate::renders::{Ask, Settings};

/// How a client names the render.
pub(super) const RENDER: &str = "render";

/// What the render does.
pub(super) const RENDER_SAYS: &str = "Render the whole timeline to a video file, or to a sound \
file of its mix alone, made by the server's render queue and kept for you to download. Takes \
real time, on a machine other people share — call project_describe first to check the cut is \
right, since that costs nothing. The same project, unchanged, in the same shape is rendered \
once: asking again answers with the file already made. The reply is either the download \
address, or the job making it; `jobs` says when that job is done and what it made. Download \
with an HTTP GET on this server, with the same token. Sketch and stale generated assets render \
as slug cards rather than failing.";

/// `render`'s arguments: the shape of the file, as the stdio tool spells it.
pub(super) fn render_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "project": project_property(),
            "container": {
                "type": "string",
                "description": "Container to deliver in: mp4 (the default), mkv, avi or wmv \
                                for video; mp3, wav or m4a for the soundtrack alone."
            },
            "video_codec": {
                "type": "string",
                "description": "Picture codec: h264, mpeg4 or wmv2. Defaults to what the \
                                container is written with; a pairing scorsese does not write \
                                is refused before anything is queued."
            },
            "audio_codec": {
                "type": "string",
                "description": "Sound codec: aac, pcm_s16le, wmav2 or mp3. Defaults to what \
                                the container is written with."
            },
            "resolution": {
                "type": "string",
                "description": "Output size, e.g. 1920x1080 (the default). Refused for a \
                                sound-only container, which has no picture to size."
            }
        },
        "required": ["project"]
    })
}

/// How a client names the job listing.
pub(super) const JOBS: &str = "jobs";

/// What the job listing does.
pub(super) const JOBS_SAYS: &str = "Say where your long-running work is: renders and \
generations, waiting, running, done — with what each made — failed with why, or stuck \
waiting on a provider. Give a job id, as render and generate answer with, for that one job; \
leave it out for your latest twenty. Read-only and free: call it as often as it takes.";

/// `jobs`' arguments.
pub(super) fn jobs_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "job": {
                "type": "integer",
                "description": "One job, by the id render or generate answered with."
            }
        }
    })
}

/// Ask for a render.
pub(super) async fn render(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let project = project_id(arguments)?;
    let mut shape = arguments.clone();
    if let Some(fields) = shape.as_object_mut() {
        fields.remove("project");
    }
    let asked: Ask = serde_json::from_value(shape)
        .map_err(|error| format!("the arguments do not read: {error}"))?;
    let settings = Settings::from_ask(&asked)?;
    let toolbox = caller.toolbox;
    let asked = ask(
        &toolbox.pool,
        &toolbox.queue,
        &toolbox.renders,
        caller.user,
        project,
        settings,
    )
    .await
    .map_err(|error| match error {
        AskError::Invalid(why) => why,
        AskError::Project(ProjectError::NotFound) => "there is no such project of yours".into(),
        other => database(other),
    })?;
    Ok(match asked {
        Asked::Kept(view) => format!(
            "Already rendered, {} bytes: GET {} to download it.",
            view.size,
            view.file()
        ),
        Asked::Queued(job) => format!(
            "Rendering as job {}, {}. Call jobs with job: {} to see when it is done; its \
             result names where to download the file.",
            job.id,
            state(&job),
            job.id
        ),
    }
    .into())
}

/// Where the caller's jobs are.
pub(super) async fn jobs(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let pool = &caller.toolbox.pool;
    let listed = match arguments.get("job").and_then(Value::as_i64) {
        Some(id) => vec![
            jobs::get(pool, caller.user, id)
                .await
                .map_err(database)?
                .ok_or_else(|| format!("there is no job {id} of yours"))?,
        ],
        None => {
            let mut all = jobs::list(pool, caller.user).await.map_err(database)?;
            all.truncate(20);
            all
        }
    };
    if listed.is_empty() {
        return Ok("You have no jobs yet.".into());
    }
    Ok(listed
        .iter()
        .map(line)
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

/// One job as a line.
fn line(job: &JobView) -> String {
    let mut said = format!("job {} ({}): {}", job.id, job.kind, state(job));
    if let Some(result) = &job.result {
        said.push_str(&format!(" — {result}"));
    }
    if let Some(error) = &job.error {
        said.push_str(&format!(" — {error}"));
    }
    said
}

/// A job's state in words.
fn state(job: &JobView) -> &'static str {
    match job.state {
        State::Waiting => "waiting its turn",
        State::Running => "running",
        State::Done => "done",
        State::Failed => "failed",
        State::Stuck => "stuck waiting on the provider; its ticket is kept",
    }
}
