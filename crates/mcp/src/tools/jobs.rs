//! `jobs`: how far a render has got, and stopping it (#700).
//!
//! `render` answers with a job id and renders on in the background, so these
//! are how an assistant learns how it went — and tells the person waiting
//! "it's at 63%, drawing frame 1190 of 1890". The names, the `job` argument
//! and the line each job is described in are the web's (`scorsese-server`'s
//! own `jobs`), so a habit learned on one surface works on the other. They
//! differ in one argument: here it takes `project`, as every tool on this
//! server does, and answers about that project's renders.
//!
//! Stopping one is the same tool's `cancel` argument rather than a tool of its
//! own (#783): following a render and stopping it are one subject, and one
//! tool fewer is one choice fewer for whoever reads the list.
//!
//! A job lives as long as the server does. The renders are this session's
//! (`crate::renders`), so a restarted server has none, and a render still
//! running when the client disconnects is stopped and its file removed.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::renders::{State, line};
use crate::tools::args::{self, ProjectDir};
use crate::tools::{Context, Costs, Reply, Tool};

/// How many jobs a listing shows: the web's twenty.
const LISTED: usize = 20;

/// Where this session's renders are.
pub(crate) struct Jobs;

/// What `jobs` takes.
#[derive(Deserialize, JsonSchema)]
struct JobsArguments {
    project: ProjectDir,
    /// One job, by the id render answered with.
    job: Option<u64>,
    /// Stop this job, by the id render answered with: a running render stops
    /// within a frame and keeps no file. One that already finished is left as
    /// it is and described instead.
    cancel: Option<u64>,
}

impl args::Arguments for JobsArguments {}

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
         latest twenty. Give `cancel` a job id to stop that render instead: \
         a running one stops within a frame and keeps no file — the partial one \
         is removed — and one that already finished is left as it is. Renders \
         belong to this server session: a restarted server has none. Free: \
         call it as often as it takes."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<JobsArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let renders = crate::renders::Renders::default();
        let cancel = scorsese_render::Cancel::new();
        self.call_in(arguments, &mut Context::new(&cancel, &renders, None))
    }

    fn call_in(&self, arguments: &Value, context: &mut Context<'_>) -> Result<Reply, String> {
        let arguments: JobsArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        if let Some(id) = arguments.cancel {
            return cancelled(context, dir, id);
        }
        let listed = match arguments.job {
            Some(id) => vec![found(context, dir, id)?],
            None => context.renders.of(dir).into_iter().take(LISTED).collect(),
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

/// Stop the job called `id`, or say how it ended when it already has.
fn cancelled(context: &Context<'_>, dir: &std::path::Path, id: u64) -> Result<Reply, String> {
    let job = found(context, dir, id)?;
    Ok(match job.state() {
        State::Running(_) => {
            job.cancel();
            format!("Stopping job {id}; it will say cancelled in a moment.")
        }
        _ => line(&job),
    }
    .into())
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
