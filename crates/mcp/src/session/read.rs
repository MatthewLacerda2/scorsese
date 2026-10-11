//! The reading thread: every line, as it arrives.

use std::collections::HashMap;
use std::io::BufRead;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use scorsese_render::Cancel;
use serde_json::Value;

use super::Event;

/// The calls read but not yet answered, by the id the client gave them —
/// written as JSON, since an id may be a number or a string and `1` and `"1"`
/// are different ids.
pub(super) type InFlight = Arc<Mutex<HashMap<String, Cancel>>>;

/// Reads `input` to its end, sending each line on — a cancel excepted, which
/// is acted on here and goes no further.
///
/// A call is registered under its id *here*, before it is sent, so that a
/// cancel read a moment later finds it whether or not a worker has started on
/// it yet. One that has not starts already stopped, which is the cheapest
/// render there is.
pub(super) fn read(input: impl BufRead, in_flight: &InFlight, send: &Sender<Event>) {
    for line in input.lines() {
        let line = match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => line,
            Err(problem) => {
                let _ = send.send(Event::Ended(Err(problem)));
                return;
            }
        };
        let message: Option<Value> = serde_json::from_str(&line).ok();
        let method = message.as_ref().and_then(|m| m.get("method"));
        let call = match method.and_then(Value::as_str) {
            Some("notifications/cancelled") => {
                let named = message
                    .as_ref()
                    .and_then(|m| m.pointer("/params/requestId"))
                    .map(Value::to_string);
                if let Some(cancel) = named.and_then(|id| lock(in_flight).get(&id).cloned()) {
                    cancel.cancel();
                }
                continue;
            }
            Some("tools/call") => message
                .as_ref()
                .and_then(|m| m.get("id"))
                .map(|id| (id.to_string(), Cancel::new())),
            _ => None,
        };
        if let Some((id, cancel)) = &call {
            lock(in_flight).insert(id.clone(), cancel.clone());
        }
        if send.send(Event::Line(line, call)).is_err() {
            return;
        }
    }
    let _ = send.send(Event::Ended(Ok(())));
}

/// The table of calls in flight. A panic while it was held — there is no code
/// under the lock that can panic — would leave it poisoned but intact, so the
/// map is used as it stands.
pub(super) fn lock(in_flight: &InFlight) -> MutexGuard<'_, HashMap<String, Cancel>> {
    in_flight.lock().unwrap_or_else(PoisonError::into_inner)
}
