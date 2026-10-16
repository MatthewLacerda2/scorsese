//! `render` and `jobs`: work the queue does, asked for, asked after and
//! stopped.
//!
//! Locally `render` runs on its own thread in the session and answers at once
//! with a job, which the local `jobs` follows and stops (#700). On the
//! server a render is a job too (#541) — the machine is shared, and
//! two at once is what it carries — and its file is kept in the render cache
//! for download. So `render` asks for one exactly as `POST
//! /api/projects/{id}/renders` does ([`crate::renders::request`]), and `jobs`
//! is how a client learns it finished, as it is for a generation.
//!
//! Locally a client can also wait on a render (`wait: true`) and stop it by
//! cancelling that call (`notifications/cancelled`, #647). Here the call
//! answers the moment the job is queued, so there is nothing left for the
//! notification to stop, and `jobs`' `cancel` is the same "stop" said about
//! the job (#660, a tool of its own until #783) —
//! [`crate::jobs::cancel`], exactly as `POST /api/jobs/{id}/cancel` says it.
//!
//! What it does not do is hold the call for the whole job. `jobs`' `wait`
//! (#1301) is the local tool's, bounded at a minute: [`wait`] has why.

mod render;
mod wait;

use scorsese_mcp::Reply;
use serde_json::{Value, json};

use super::super::{Caller, database};
use crate::jobs::{CancelError, JobView, ProgressView, State, store as jobs};

pub(super) use render::{RENDER, RENDER_SAYS, render, render_schema};

/// How a client names the job listing.
pub(super) const JOBS: &str = "jobs";

/// What the job listing does.
pub(super) const JOBS_SAYS: &str = "Say where your long-running work is: renders and \
generations, waiting, running, done — with what each made — failed with why, or stuck \
waiting on a provider. A running render says how far it has got, as a percentage of its \
frames and what it is doing (preparing, capturing the web pages, mixing the sound, drawing \
frames, finishing the file), so \"how far is my render?\" has a number. Give a job id, as render and generate \
answer with, for that one job; leave it out for your latest twenty. Give `cancel` a job id to \
stop that render instead: a waiting one never starts, a running one stops within a frame and \
keeps no file. Only renders can be stopped — a generation is billed whether or not anybody \
still wants it — and a job that already finished is left as it is. A queued generation or \
render lands in the project on its own when it finishes, and a shot takes minutes: check a \
job when the person asks about it or a later step needs what it made, never over and over to \
wait it out — the call costs nothing here, but every check is a full round trip for whoever \
is calling. Leave it running and say it is on its way. When a later step really needs what \
a job made, add wait: true to its job id: the call answers as soon as that job is done, \
failed, stuck or cancelled, or after a minute at most with how it stands then — wait again \
if it is still going.";

/// `jobs`' arguments.
pub(super) fn jobs_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "job": {
                "type": "integer",
                "description": "One job, by the id render or generate answered with."
            },
            "cancel": {
                "type": "integer",
                "description": "Stop this render, by the id render answered with. One that \
                                already finished is left as it is and described instead."
            },
            "wait": {
                "type": "boolean",
                "description": "true, with `job`, to answer only once that job has ended — \
                                done, failed, stuck or cancelled — or after a minute at most, \
                                with its line then either way. One already ended answers at \
                                once. Default false: answer at once."
            }
        }
    })
}

/// Stop the job called `id`.
async fn cancel(caller: &Caller<'_>, id: i64) -> Result<Reply, String> {
    let toolbox = caller.toolbox;
    let job = crate::jobs::cancel(&toolbox.pool, &toolbox.queue, caller.user, id)
        .await
        .map_err(|error| match error {
            CancelError::Database(error) => database(error),
            refused => refused.to_string(),
        })?;
    Ok(match job.state {
        State::Running => format!("Stopping job {id}; it will say cancelled in a moment."),
        _ => line(&job),
    }
    .into())
}

/// Where the caller's jobs are.
pub(super) async fn jobs(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    if let Some(cancelling) = arguments.get("cancel") {
        let id = cancelling
            .as_i64()
            .ok_or("`cancel` is a job id: the number render answered with")?;
        return cancel(caller, id).await;
    }
    if arguments.get("wait").and_then(Value::as_bool) == Some(true) {
        let id = arguments.get("job").and_then(Value::as_i64).ok_or(
            "wait waits for one job: give job, the id render or generate answered with — \
             jobs without it lists them all, and waiting on all of them has no end",
        )?;
        return wait::waited(caller, id).await;
    }
    let pool = &caller.toolbox.pool;
    let listed: Vec<JobView> = match arguments.get("job").and_then(Value::as_i64) {
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
    let queue = &caller.toolbox.queue;
    Ok(listed
        .into_iter()
        .map(|job| line(&queue.progressed(job)))
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

/// One job as a line: `job 12 (render): running — 42% (760 of 1800 frames)`.
///
/// The shape is the answer's contract (#698): `job {id} ({kind}): {state}`,
/// then ` — ` and how far it has got while it runs, then ` — ` and what it made
/// or why it did not.
fn line(job: &JobView) -> String {
    let mut said = format!("job {} ({}): {}", job.id, job.kind, state(job));
    if let Some(progress) = &job.progress {
        said.push_str(&format!(" — {}", progressed(progress)));
    }
    if let Some(result) = &job.result {
        said.push_str(&format!(" — {result}"));
    }
    if let Some(error) = &job.error {
        said.push_str(&format!(" — {error}"));
    }
    said
}

/// How far a running job has got, in words. The phase is said wherever the
/// frames say nothing, so 0% while mixing does not read as stuck.
fn progressed(progress: &ProgressView) -> String {
    let percent = progress.percent;
    match progress.phase {
        "drawing" if progress.of > 0 => {
            format!("{percent}% ({} of {} frames)", progress.done, progress.of)
        }
        "preparing" => format!("{percent}%, preparing"),
        "mixing" => format!("{percent}%, mixing the sound"),
        "finishing" => format!("{percent}%, finishing the file"),
        _ => format!("{percent}%"),
    }
}

/// A job's state in words.
fn state(job: &JobView) -> &'static str {
    match job.state {
        State::Waiting => "waiting its turn",
        State::Running => "running",
        State::Done => "done",
        State::Failed => "failed",
        State::Stuck => "stuck waiting on the provider; its ticket is kept",
        State::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running(progress: Option<ProgressView>) -> JobView {
        JobView {
            id: 12,
            kind: "render".into(),
            state: State::Running,
            attempts: 1,
            result: None,
            error: None,
            created_at: 0,
            started_at: Some(1),
            finished_at: None,
            interrupted_at: None,
            progress,
        }
    }

    fn at(percent: u8, phase: &'static str, done: u64, of: u64) -> Option<ProgressView> {
        Some(ProgressView {
            percent,
            phase,
            done,
            of,
        })
    }

    #[test]
    fn a_running_render_says_its_percentage_and_frames() {
        assert_eq!(
            line(&running(at(42, "drawing", 760, 1800))),
            "job 12 (render): running — 42% (760 of 1800 frames)"
        );
        assert_eq!(line(&running(None)), "job 12 (render): running");
    }

    #[test]
    fn the_phase_is_said_where_the_frames_say_nothing() {
        let said = |progress| line(&running(progress));
        assert!(said(at(0, "preparing", 0, 0)).ends_with("— 0%, preparing"));
        assert!(said(at(0, "mixing", 0, 90)).ends_with("— 0%, mixing the sound"));
        assert!(said(at(99, "finishing", 90, 90)).ends_with("— 99%, finishing the file"));
        assert!(said(at(0, "drawing", 0, 0)).ends_with("— 0%"));
        assert!(said(at(100, "done", 90, 90)).ends_with("— 100%"));
    }
}
