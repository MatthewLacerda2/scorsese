//! The stdio transport: a line in, a line out.
//!
//! What each line *means* is [`protocol`]'s; this is only the framing, and
//! running each tool against the project directory it names.
//!
//! ## Calls run side by side (#1063)
//!
//! A client does not wait on one answer before sending the next request.
//! Claude Code moves a call to the background after two minutes and carries
//! on, so a `generate` paying for eighteen stills used to stall the reads
//! behind it — a `synth_kit` looking up a table compiled into this binary
//! answered only when the provider had. So every `tools/call` runs on one of
//! [`WORKERS`] threads, and answers when it is done, in whatever order that
//! is: JSON-RPC matches an answer to its request by id, never by position.
//! Everything else — `initialize`, `ping`, `tools/list`, a line that is not
//! JSON — is answered here at once, in the order it arrived.
//!
//! **Calls that change a project take turns at it** ([`hold`]). Two calls each
//! loading `project.json`, changing it and saving the whole document back
//! would have the later one refused as stale at best, and a `generate` holding
//! a document for minutes is the call most worth not refusing: its save is the
//! record of what was paid for. So a call that may change a project holds it
//! for the whole call, and one that only reads never waits.
//!
//! ## The reader, because a client may say "stop"
//!
//! The reading thread looks at each line as it arrives ([`read`]): a
//! `notifications/cancelled` trips the [`Cancel`] of the call it names, which
//! is how a `render` asked to `wait` is stopped mid-way (#647). A `render` that
//! is not asked to wait answers at once and renders on its own thread, one of
//! the session's renders: they are held here, and stopped when the session
//! ends (#700). And a call that carries a `progressToken` writes
//! `notifications/progress` before its reply, which is why a call is handed
//! somewhere to say things.
//!
//! A cancelled call gets **no reply**, as the specification asks of a receiver.
//! What it would have said — a render's "cancelled after 412 of 1890 frames" —
//! goes to stderr instead, which a client shows as the server's log.

mod hold;
mod read;
mod run;
#[cfg(test)]
mod tests;

use std::io::{BufRead, Write};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use scorsese_render::Cancel;
use serde_json::Value;

use crate::renders::Renders;
use hold::Holds;
use read::{InFlight, read};
use run::Job;

/// How many calls run at once.
///
/// Enough that the slow calls a client can leave running in the background
/// all at the same time — a `generate`, a `synth_bake`, a `render` asked to
/// wait, a `still` capturing a page, a `stock_search` — still leave room for
/// the reads it goes on making. Not unbounded, because each of those may hold
/// a process or a core of its own, and a client that fires fifty frames at
/// once should have them queue here rather than make the machine thrash.
/// Threads waiting for work cost nothing.
const WORKERS: usize = 8;

/// What the serving loop hears about: lines read, and what calls say.
enum Event {
    /// A line from the client, with the cancel its call runs under when it is
    /// a call.
    Line(String, Option<(String, Cancel)>),
    /// The input ended, or could not be read.
    Ended(std::io::Result<()>),
    /// A line for the client that is not a call's last word — progress.
    Said(String),
    /// A call is over, with its reply unless it is to get none.
    Done(Option<Value>),
}

/// Everything a call runs beside: the session's renders, which projects are
/// being changed, and which calls may still be stopped.
struct Shared {
    renders: Renders,
    holds: Holds,
    in_flight: InFlight,
}

/// Reads requests from `input` until it ends, writing a reply to `output` for
/// each one that is not a notification and was not cancelled — returning once
/// every call it read has answered.
///
/// A malformed line is answered and the loop carries on. A client that sends
/// one line of nonsense has not ended the conversation, and a server that
/// exits on it would take a whole session down over a typo.
pub fn serve(input: impl BufRead + Send + 'static, mut output: impl Write) -> std::io::Result<()> {
    // Its renders are dropped last, whichever way this returns: those still
    // running are stopped and their files removed (#700).
    let shared = Shared {
        renders: Renders::default(),
        holds: Holds::default(),
        in_flight: InFlight::default(),
    };
    let (events, inbox) = channel();
    // Detached rather than scoped: should answering fail — the client gone —
    // this returns without it, and a reader blocked on stdin must not hold it.
    // It ends by itself at the next line, when there is nobody to send it to.
    let reading = Arc::clone(&shared.in_flight);
    let heard = events.clone();
    std::thread::spawn(move || read(input, &reading, &heard));

    let (work, jobs) = channel::<Job>();
    let jobs = Mutex::new(jobs);
    std::thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| run::work(&jobs, &events, &shared));
        }
        let served = pump(&inbox, &work, &mut output, &shared);
        if served.is_err() {
            // Nobody is left to answer: stop what can be stopped, so the
            // workers this waits for below are not a render's hours.
            for cancel in read::lock(&shared.in_flight).values() {
                cancel.cancel();
            }
        }
        drop(work);
        served
    })
}

/// The serving loop: hands calls to the workers, answers everything else, and
/// writes every line anyone has for the client.
fn pump(
    inbox: &Receiver<Event>,
    work: &Sender<Job>,
    output: &mut impl Write,
    shared: &Shared,
) -> std::io::Result<()> {
    let mut reading = true;
    let mut running = 0_usize;
    while reading || running > 0 {
        let Ok(event) = inbox.recv() else {
            break;
        };
        match event {
            Event::Line(line, Some(call)) => {
                running += 1;
                if work.send(Job { line, call }).is_err() {
                    break;
                }
            }
            Event::Line(line, None) => {
                let mut say = |note: String| {
                    // Best effort, as a worker's progress is: a pipe that
                    // will not take this will not take the reply either.
                    let _ = writeln!(output, "{note}").and_then(|()| output.flush());
                };
                if let Some(reply) = run::handle(&line, None, shared, &mut say) {
                    write(output, &reply)?;
                }
            }
            Event::Ended(ended) => {
                ended?;
                reading = false;
            }
            Event::Said(note) => {
                writeln!(output, "{note}")?;
                output.flush()?;
            }
            Event::Done(reply) => {
                running -= 1;
                if let Some(reply) = reply {
                    write(output, &reply)?;
                }
            }
        }
    }
    Ok(())
}

/// Writes one reply as one line, flushed: a client may be waiting on exactly
/// this answer, and a buffered reply is a hung session.
fn write(output: &mut impl Write, reply: &Value) -> std::io::Result<()> {
    let encoded = serde_json::to_string(reply)
        .unwrap_or_else(|_| r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"could not encode the reply"}}"#.to_owned());
    writeln!(output, "{encoded}")?;
    output.flush()
}
