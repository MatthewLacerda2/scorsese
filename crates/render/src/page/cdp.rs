//! The Chrome DevTools Protocol, spoken over a pair of pipes.
//!
//! Hand-written, and deliberately small: a capture needs about a dozen methods,
//! and every CDP crate brings an async runtime and a websocket stack to reach
//! a browser that is already our child process on two file descriptors (#606).
//! The wire format over `--remote-debugging-pipe` is one JSON object per
//! message, each terminated by a NUL byte, in both directions.
//!
//! The client is synchronous. A command is written, and messages are read until
//! its answer arrives; every **event** read on the way is handed to a
//! [`Listener`], which may answer it with a command of its own. That is how a
//! paused request is fulfilled while a navigation is still waiting on it — the
//! browser cannot finish loading a page until we serve the page, so the
//! answering has to happen inside the wait.
//!
//! Generic over the writer and fed parsed messages through a channel, so all
//! of it is tested here without a browser.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use serde_json::{Value, json};

/// A command for the browser: a method, its parameters, and which page session
/// it is for (none for the browser itself).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Command {
    pub(crate) method: &'static str,
    pub(crate) params: Value,
    pub(crate) session: Option<String>,
}

/// Whoever is interested in what the browser says unprompted.
pub(crate) trait Listener {
    /// One event, as it arrived. Whatever is returned is sent straight back,
    /// unawaited: an answer to a paused request is not itself waited on.
    fn heard(&mut self, method: &str, params: &Value, session: Option<&str>) -> Option<Command>;
}

/// Why talking to the browser stopped working.
#[derive(Debug, thiserror::Error)]
pub enum CdpError {
    /// The browser closed its end, which almost always means it crashed or was
    /// killed.
    #[error("the browser closed the connection")]
    Closed,
    /// Nothing came back in time. A page that loops forever in a script is the
    /// usual cause, since a page's script runs on the thread that answers.
    #[error("the browser did not answer `{0}` in time")]
    Timeout(String),
    /// The browser understood the command and refused it.
    #[error("the browser refused `{method}`: {message}")]
    Refused {
        /// The command it refused.
        method: String,
        /// Its own explanation.
        message: String,
    },
    /// Writing to the pipe failed.
    #[error("writing to the browser: {0}")]
    Io(#[from] std::io::Error),
}

/// One connection: a writer to the browser, and its messages as they arrive.
pub(crate) struct Cdp<W: Write, L: Listener> {
    writer: W,
    incoming: Receiver<Value>,
    listener: L,
    next_id: u64,
    /// Commands sent on a listener's behalf, whose answers nobody is waiting
    /// for and which are dropped when they arrive.
    unawaited: HashSet<u64>,
    timeout: Duration,
}

impl<W: Write, L: Listener> Cdp<W, L> {
    /// A connection that gives up on any one answer after `timeout`.
    pub(crate) fn new(
        writer: W,
        incoming: Receiver<Value>,
        listener: L,
        timeout: Duration,
    ) -> Self {
        Self {
            writer,
            incoming,
            listener,
            next_id: 1,
            unawaited: HashSet::new(),
            timeout,
        }
    }

    /// The listener, for what it has heard so far.
    pub(crate) const fn listener(&self) -> &L {
        &self.listener
    }

    /// Sends `method` to the browser itself, and waits for its answer.
    pub(crate) fn browser(
        &mut self,
        method: &'static str,
        params: Value,
    ) -> Result<Value, CdpError> {
        self.call(&Command {
            method,
            params,
            session: None,
        })
    }

    /// Sends a command and waits for its answer, handing every event that
    /// arrives meanwhile to the listener.
    pub(crate) fn call(&mut self, command: &Command) -> Result<Value, CdpError> {
        let id = self.send(command)?;
        loop {
            let message = self.next(command.method)?;
            if message.get("id").and_then(Value::as_u64) == Some(id) {
                return answer(command.method, message);
            }
        }
    }

    /// Reads messages until `done` says the listener has heard enough.
    pub(crate) fn until(&mut self, what: &str, done: impl Fn(&L) -> bool) -> Result<(), CdpError> {
        while !done(&self.listener) {
            self.next(what)?;
        }
        Ok(())
    }

    fn send(&mut self, command: &Command) -> Result<u64, CdpError> {
        let id = self.next_id;
        self.next_id += 1;
        let mut message = json!({ "id": id, "method": command.method, "params": command.params });
        if let Some(session) = &command.session {
            message["sessionId"] = json!(session);
        }
        let mut bytes = serde_json::to_vec(&message).expect("a JSON value always serialises");
        bytes.push(0);
        self.writer.write_all(&bytes)?;
        self.writer.flush()?;
        Ok(id)
    }

    /// The next message that is not an answer to an unawaited command. Events
    /// are handed to the listener here, and its reply sent, before the event
    /// is returned to whoever was waiting.
    fn next(&mut self, waiting_on: &str) -> Result<Value, CdpError> {
        loop {
            let message = match self.incoming.recv_timeout(self.timeout) {
                Ok(message) => message,
                Err(RecvTimeoutError::Timeout) => {
                    return Err(CdpError::Timeout(waiting_on.to_owned()));
                }
                Err(RecvTimeoutError::Disconnected) => return Err(CdpError::Closed),
            };
            if let Some(id) = message.get("id").and_then(Value::as_u64) {
                if self.unawaited.remove(&id) {
                    continue;
                }
                return Ok(message);
            }
            if let Some(method) = message.get("method").and_then(Value::as_str) {
                let params = message.get("params").cloned().unwrap_or(Value::Null);
                let session = message.get("sessionId").and_then(Value::as_str);
                if let Some(reply) = self.listener.heard(method, &params, session) {
                    let id = self.send(&reply)?;
                    self.unawaited.insert(id);
                }
            }
            return Ok(message);
        }
    }
}

/// An answer's result, or the refusal it carries instead.
fn answer(method: &str, message: Value) -> Result<Value, CdpError> {
    if let Some(error) = message.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("no reason given");
        return Err(CdpError::Refused {
            method: method.to_owned(),
            message: message.to_owned(),
        });
    }
    Ok(message.get("result").cloned().unwrap_or(Value::Null))
}

/// Reads NUL-terminated JSON messages from `from` until it closes, sending each
/// one on. Runs on a thread of its own: the browser writes whenever it likes,
/// and a pipe nobody reads fills up and stalls it.
///
/// A message that does not parse is dropped rather than ending the
/// connection — the browser does not send one, and if it ever did the command
/// waiting on it times out with a name to report, which says more than a
/// parse error would.
pub(crate) fn read_messages(from: impl Read, to: &Sender<Value>) {
    let mut reader = BufReader::new(from);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(0, &mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if buffer.last() == Some(&0) {
            buffer.pop();
        }
        if let Ok(message) = serde_json::from_slice(&buffer)
            && to.send(message).is_err()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    /// Remembers every event, and answers a paused request with a command.
    #[derive(Default)]
    struct Ear {
        heard: Vec<String>,
    }

    impl Listener for Ear {
        fn heard(
            &mut self,
            method: &str,
            params: &Value,
            session: Option<&str>,
        ) -> Option<Command> {
            self.heard.push(method.to_owned());
            (method == "Fetch.requestPaused").then(|| Command {
                method: "Fetch.failRequest",
                params: json!({ "requestId": params["requestId"] }),
                session: session.map(str::to_owned),
            })
        }
    }

    fn sent(bytes: &[u8]) -> Vec<Value> {
        bytes
            .split(|byte| *byte == 0)
            .filter(|message| !message.is_empty())
            .map(|message| serde_json::from_slice(message).unwrap())
            .collect()
    }

    fn connection(messages: Vec<Value>) -> Cdp<Vec<u8>, Ear> {
        let (to, from) = mpsc::channel();
        for message in messages {
            to.send(message).unwrap();
        }
        Cdp::new(Vec::new(), from, Ear::default(), Duration::from_millis(50))
    }

    #[test]
    fn a_command_is_one_nul_terminated_message_naming_its_session() {
        let mut cdp = connection(vec![json!({ "id": 1, "result": { "ok": true } })]);
        let answer = cdp
            .call(&Command {
                method: "Page.enable",
                params: json!({}),
                session: Some("s1".into()),
            })
            .unwrap();
        assert_eq!(answer, json!({ "ok": true }));
        assert_eq!(cdp.writer.last(), Some(&0));
        assert_eq!(
            sent(&cdp.writer),
            [json!({ "id": 1, "method": "Page.enable", "params": {}, "sessionId": "s1" })]
        );
    }

    #[test]
    fn events_before_the_answer_reach_the_listener_and_its_reply_is_sent_unawaited() {
        let mut cdp = connection(vec![
            json!({ "method": "Fetch.requestPaused", "params": { "requestId": "r7" }, "sessionId": "s1" }),
            // The answer to the listener's own reply, which nobody waits on.
            json!({ "id": 2, "result": {} }),
            json!({ "id": 1, "result": { "frameId": "f" } }),
        ]);
        let answer = cdp.browser("Page.navigate", json!({ "url": "x" })).unwrap();
        assert_eq!(answer, json!({ "frameId": "f" }));
        assert_eq!(cdp.listener().heard, ["Fetch.requestPaused"]);
        let sent = sent(&cdp.writer);
        assert_eq!(sent[1]["method"], "Fetch.failRequest");
        assert_eq!(sent[1]["params"]["requestId"], "r7");
        assert_eq!(sent[1]["sessionId"], "s1");
    }

    #[test]
    fn a_refusal_names_the_method_and_the_reason() {
        let mut cdp = connection(vec![
            json!({ "id": 1, "error": { "code": -32000, "message": "nope" } }),
        ]);
        let error = cdp.browser("Target.createTarget", json!({})).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the browser refused `Target.createTarget`: nope"
        );
    }

    #[test]
    fn silence_is_a_timeout_and_a_closed_pipe_is_closed() {
        let (to, from) = mpsc::channel();
        let mut cdp = Cdp::new(Vec::new(), from, Ear::default(), Duration::from_millis(20));
        assert!(matches!(cdp.browser("A.b", json!({})), Err(CdpError::Timeout(m)) if m == "A.b"));
        drop(to);
        assert!(matches!(
            cdp.browser("A.b", json!({})),
            Err(CdpError::Closed)
        ));
    }

    #[test]
    fn waiting_for_an_event_reads_until_the_listener_has_it() {
        let mut cdp = connection(vec![
            json!({ "method": "Network.requestWillBeSent", "params": {} }),
            json!({ "method": "Page.loadEventFired", "params": {} }),
        ]);
        cdp.until("load", |ear| {
            ear.heard.iter().any(|m| m == "Page.loadEventFired")
        })
        .unwrap();
        assert_eq!(cdp.listener().heard.len(), 2);
    }

    #[test]
    fn messages_are_split_on_nul_and_garbage_is_skipped() {
        let (to, from) = mpsc::channel();
        read_messages(&b"{\"id\":1}\0not json\0{\"id\":2}"[..], &to);
        drop(to);
        let ids: Vec<_> = from.iter().map(|m| m["id"].as_u64().unwrap()).collect();
        assert_eq!(ids, [1, 2]);
    }
}
