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
//! Waiting for one is the same tool's `wait` argument (#1056): `render`
//! answers at once by default, and an assistant left with nothing to do but
//! wait for the file must never have to leave the tools to do it — on
//! 2026-10-10 one improvised a shell loop that never ended, and sat three hours
//! past a finished render. It is `render`'s own `wait` reached from a job id
//! ([`crate::renders::await_end`]), progress notifications included; cancelling
//! the waiting call ends the wait and leaves the render running.
//!
//! The web's `jobs` has no `wait`, as its `render` has none: there a job is a
//! row the queue works through, and a call held open on it would hold an HTTP
//! request and a chat turn for as long as the render takes. Its assistant has
//! no shell to wander off to either; it asks `jobs` again.
//!
//! A job lives as long as the server does. The renders are this session's
//! (`crate::renders`), so a restarted server has none, and a render still
//! running when the client disconnects is stopped and its file removed.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::renders::{Say, State, await_end, line};
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
    /// true, with `job`, to answer only once that job is done, failed or
    /// cancelled — its line then, as without `wait` — sending MCP progress
    /// notifications on the way when the call carries a progressToken. For a
    /// render started without wait that you now have nothing to do but wait
    /// for. One already finished answers at once. Cancelling this call stops
    /// the waiting, not the render. Default false: answer at once.
    #[serde(default)]
    wait: bool,
}

impl args::Arguments for JobsArguments {}

impl Tool for Jobs {
    fn name(&self) -> &'static str {
        "jobs"
    }

    fn description(&self) -> &'static str {
        "Say how far your renders of a project have got, and what each one \
         made. A running render says its percentage and what is happening — \
         preparing, capturing the web pages (page frame 120 of 5900), mixing \
         the sound, drawing frame 1190 of 1890, finishing the file; a done one \
         says the file it wrote; a failed one says why. The percentage counts \
         frames, so it sits at 0 while pages are captured and the sound mixed and \
         at 99 while the file is finished: read the words beside it. Give a job \
         id, as render answers with, for that one job; leave it out for the \
         latest twenty. Add wait: true to a job id to answer only when that job has \
         ended — done, failed or cancelled — instead of asking again and again. Give `cancel` a job id to stop that render instead: \
         a running one stops within a frame and keeps no file — the partial one \
         is removed — and one that already finished is left as it is. Renders \
         belong to this server session: a restarted server has none. Free: \
         call it as often as it takes."
    }

    fn writes(&self) -> bool {
        false
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
        if arguments.wait {
            let id = arguments.job.ok_or(
                "wait waits for one job: give job, the id render answered with — \
                 jobs without it lists them all, and waiting on all of them has no end",
            )?;
            return waited(context, dir, id);
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

/// The job called `id`'s line once it has ended, reporting how far it has got
/// on the way; at once when it already has.
fn waited(context: &mut Context<'_>, dir: &std::path::Path, id: u64) -> Result<Reply, String> {
    let job = found(context, dir, id)?;
    let report = context
        .report
        .as_mut()
        .map(|report| &mut **report as &mut Say<'_>);
    Ok(match await_end(&job, context.cancel, report) {
        Some(_) => line(&job),
        // Not answered — the client cancelled the call — but said to the log.
        None => format!("Stopped waiting for job {id}; it renders on."),
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

#[cfg(test)]
mod tests;
