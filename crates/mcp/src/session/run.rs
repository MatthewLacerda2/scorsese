//! Running one line: a worker's loop, and what a line is answered with.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Mutex, PoisonError};

use scorsese_render::Cancel;
use serde_json::Value;

use super::{Event, Shared, read};
use crate::protocol::{self, Call, Handled};
use crate::renders::Say;
use crate::tools::{self, Context, Tool};

/// A call for a worker: its line, its id and the cancel it runs under.
pub(super) struct Job {
    pub(super) line: String,
    pub(super) call: (String, Cancel),
}

/// One worker: takes calls until there are no more to take, and says what
/// each one answered.
pub(super) fn work(jobs: &Mutex<Receiver<Job>>, events: &Sender<Event>, shared: &Shared) {
    loop {
        // The receiver is only ever held while waiting for the next job, so a
        // poisoned lock is one nothing was half-way through.
        let job = jobs.lock().unwrap_or_else(PoisonError::into_inner).recv();
        let Ok(Job { line, call }) = job else {
            return;
        };
        let (id, cancel) = call;
        let mut say = |note: String| {
            let _ = events.send(Event::Said(note));
        };
        let reply = handle(&line, Some(&cancel), shared, &mut say);
        read::lock(&shared.in_flight).remove(&id);
        let reply = if cancel.is_cancelled() {
            if let Some(said) = reply.as_ref().and_then(said) {
                eprintln!("scorsese-mcp: request {id} was cancelled: {said}");
            }
            None
        } else {
            reply
        };
        if events.send(Event::Done(reply)).is_err() {
            return;
        }
    }
}

/// What a reply says, as text: the words of its first block, or its error.
fn said(reply: &Value) -> Option<String> {
    reply
        .pointer("/result/content/0/text")
        .or_else(|| reply.pointer("/error/message"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// One line's reply, or `None` when the line was a notification.
///
/// `say` is for what a call says before its reply: the progress a client
/// asked for with a `progressToken`, as a whole line, written as it happens.
pub(super) fn handle(
    line: &str,
    cancel: Option<&Cancel>,
    shared: &Shared,
    say: &mut dyn FnMut(String),
) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(problem) => return Some(protocol::unreadable(problem)),
    };
    match protocol::handle(message, listed) {
        Handled::Silent => None,
        Handled::Answered(reply) => Some(reply),
        Handled::Call(call) => Some(match tools::find(&call.name) {
            Some(tool) => {
                let unnamed = Cancel::new();
                let cancel = cancel.unwrap_or(&unnamed);
                let outcome = call_held(tool.as_ref(), &call, cancel, shared, say);
                call.answer(outcome)
            }
            None => call.unknown(),
        }),
    }
}

/// Runs `tool`, holding the project it names first when it may change it.
///
/// A tool that panics is a bug in this server, and answered as one: the call
/// is refused with what it said, and the session — and every other call
/// running beside it — carries on.
fn call_held(
    tool: &dyn Tool,
    call: &Call,
    cancel: &Cancel,
    shared: &Shared,
    say: &mut dyn FnMut(String),
) -> Result<tools::Reply, String> {
    let project = call.arguments.get("project").and_then(Value::as_str);
    let _held = match project {
        Some(project) if tool.writes() => Some(shared.holds.take(project, tool.name())?),
        _ => None,
    };
    let token = call.progress_token().cloned();
    let mut notify = |progress: f64, message: &str| {
        if let Some(token) = &token {
            say(protocol::progress(token, progress, message).to_string());
        }
    };
    let report = token.is_some().then_some(&mut notify as &mut Say<'_>);
    let mut context = Context::new(cancel, &shared.renders, report).holding(&shared.holds);
    catch_unwind(AssertUnwindSafe(|| {
        tool.call_in(&call.arguments, &mut context)
    }))
    .unwrap_or_else(|panic| {
        let what = panic
            .downcast_ref::<&str>()
            .map(|said| (*said).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        Err(format!(
            "`{}` stopped on a bug in scorsese-mcp, not in what it was asked: {what}",
            tool.name()
        ))
    })
}

/// Every tool, as a client lists them.
fn listed() -> Vec<Value> {
    tools::registry()
        .iter()
        .map(|tool| protocol::listing(tool.as_ref()))
        .collect()
}
