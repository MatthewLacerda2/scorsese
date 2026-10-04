//! `jobs` and `job_cancel`: how far a render has got, and stopping it (#700).
//!
//! `render` answers with a job id and renders on in the background, so these
//! are how an assistant learns how it went — and tells the person waiting
//! "it's at 63%, drawing frame 1190 of 1890". The names, the `job` argument
//! and the line each job is described in are the web's (`scorsese-server`'s
//! own `jobs` and `job_cancel`), so a habit learned on one surface works on
//! the other. They differ in one argument: here they take `project`, as every
//! tool on this server does, and answer about that project's renders.
//!
//! A job lives as long as the server does. The renders are this session's
//! (`crate::renders`), so a restarted server has none, and a render still
//! running when the client disconnects is stopped and its file removed.

use serde_json::{Value, json};

use crate::renders::{State, line};
use crate::tools::{Context, Costs, Reply, Tool, project_dir, project_property};

/// How many jobs a listing shows: the web's twenty.
const LISTED: usize = 20;

/// Where this session's renders are.
pub(crate) struct Jobs;

impl Tool for Jobs {
    fn name(&self) -> &'static str {
        "jobs"
    }

    fn description(&self) -> &'static str {
        "Say how far your renders of a project have got, and what each one \
         made. A running render says its percentage and what is happening — \
         preparing, mixing the sound, drawing frame 1190 of 1890, finishing the \
         file; a done one says the file it wrote; a failed one says why. The \
         percentage counts frames, so it sits at 0 while the sound is mixed and \
         at 99 while the file is finished: read the words beside it. Give a job \
         id, as render answers with, for that one job; leave it out for the \
         latest twenty. Renders \
         belong to this server session: a restarted server has none. Read-only \
         and free: call it as often as it takes."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "job": {
                    "type": "integer",
                    "description": "One job, by the id render answered with."
                }
            },
            "required": ["project"]
        })
    }

    /// Outside a session there are no renders to report.
    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let renders = crate::renders::Renders::default();
        let cancel = scorsese_render::Cancel::new();
        self.call_in(arguments, &mut Context::new(&cancel, &renders, None))
    }

    fn call_in(&self, arguments: &Value, context: &mut Context<'_>) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let listed = match job(arguments) {
            Some(id) => vec![found(context, &dir, id)?],
            None => context.renders.of(&dir).into_iter().take(LISTED).collect(),
        };
        if listed.is_empty() {
            return Ok(
                "No renders of this project yet in this session — render answers \
                       with a job id to follow."
                    .into(),
            );
        }
        Ok(listed
            .iter()
            .map(|job| line(job))
            .collect::<Vec<_>>()
            .join("\n")
            .into())
    }
}

/// Stopping a render.
pub(crate) struct JobCancel;

impl Tool for JobCancel {
    fn name(&self) -> &'static str {
        "job_cancel"
    }

    fn description(&self) -> &'static str {
        "Stop one of your renders: a running one stops within a frame and keeps \
         no file — the partial one is removed. Give the job id render answered \
         with. A job that already finished is left as it is. Free."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "job": {
                    "type": "integer",
                    "description": "The job to stop, by the id render answered with."
                }
            },
            "required": ["project", "job"]
        })
    }

    /// Outside a session there are no renders to stop.
    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let renders = crate::renders::Renders::default();
        let cancel = scorsese_render::Cancel::new();
        self.call_in(arguments, &mut Context::new(&cancel, &renders, None))
    }

    fn call_in(&self, arguments: &Value, context: &mut Context<'_>) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let id = job(arguments).ok_or("`job` is required: the id render answered with")?;
        let job = found(context, &dir, id)?;
        Ok(match job.state() {
            State::Running(_) => {
                job.cancel();
                format!("Stopping job {id}; it will say cancelled in a moment.")
            }
            _ => line(&job),
        }
        .into())
    }
}

/// The `job` argument.
fn job(arguments: &Value) -> Option<u64> {
    arguments.get("job").and_then(Value::as_u64)
}

/// The job called `id`, when it is one of this session's renders of `dir`.
fn found(
    context: &Context<'_>,
    dir: &std::path::Path,
    id: u64,
) -> Result<std::sync::Arc<crate::renders::Job>, String> {
    context
        .renders
        .get(id)
        .filter(|job| job.renders(dir))
        .ok_or_else(|| format!("there is no job {id} rendering this project in this session"))
}
