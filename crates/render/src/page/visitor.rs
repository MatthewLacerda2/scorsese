//! Everything the browser says while a page is captured, and our answers.
//!
//! The page's every request arrives here paused, and is answered from the
//! project ([`super::origin`]) — which is also where the record of what it
//! loaded is kept, since that record is half of the capture's cache key. What
//! went wrong on the way — a request refused, a script that threw — becomes a
//! warning on the render: never a silent miss, because #606 found a missing font
//! falls back without a trace.

use std::collections::BTreeMap;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::cdp::{Command, Listener};
use super::origin::{Answer, answer};

/// The function the preamble calls, with the name of the API, when a page
/// opens a connection the protocol never reports (`offline.js`).
pub(crate) const OFFLINE_BINDING: &str = "__scorsese_offline";

/// What a capture has heard from the page so far.
pub(crate) struct Visitor<'a> {
    project_root: &'a Path,
    /// Every project file the page asked for, by its path, with the hash of
    /// what it was served — or `None` for a file that was not there, so that
    /// adding it later makes the capture stale too.
    pub(crate) loaded: BTreeMap<String, Option<String>>,
    /// What went wrong that the author should hear about, in plain words, each
    /// once.
    pub(crate) warnings: Vec<String>,
    /// Whether the page's `load` event has fired.
    pub(crate) loaded_event: bool,
}

impl<'a> Visitor<'a> {
    pub(crate) const fn new(project_root: &'a Path) -> Self {
        Self {
            project_root,
            loaded: BTreeMap::new(),
            warnings: Vec::new(),
            loaded_event: false,
        }
    }

    pub(crate) fn warn(&mut self, warning: String) {
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }

    fn serve(&mut self, params: &Value, session: Option<&str>) -> Command {
        let id = params["requestId"].clone();
        let url = params["request"]["url"].as_str().unwrap_or_default();
        let fulfil = |status: u16, body: &[u8]| {
            json!({
                "requestId": id,
                "responseCode": status,
                "responseHeaders": [
                    { "name": "Content-Type", "value": Answer::mime(url) },
                    // The shipped libraries are another origin, and a module
                    // or a font from one is refused without this.
                    { "name": "Access-Control-Allow-Origin", "value": "*" },
                    { "name": "Cache-Control", "value": "no-store" },
                ],
                "body": STANDARD.encode(body),
            })
        };
        let (method, params) = match answer(url, self.project_root) {
            Answer::File { path, body } => {
                self.loaded
                    .insert(path, Some(scorsese_core::hash_bytes(&body)));
                ("Fetch.fulfillRequest", fulfil(200, &body))
            }
            Answer::Shipped { body } => ("Fetch.fulfillRequest", fulfil(200, body)),
            Answer::Missing { path } => {
                self.warn(format!(
                    "the page asked for `{path}`, which is not in the project; it rendered without it"
                ));
                self.loaded.insert(path, None);
                ("Fetch.fulfillRequest", fulfil(404, b""))
            }
            Answer::Refused => {
                self.warn(format!(
                    "the page asked for {url} from outside the project; pages render offline, \
                     so it rendered without it"
                ));
                (
                    "Fetch.failRequest",
                    json!({ "requestId": id, "errorReason": "BlockedByClient" }),
                )
            }
        };
        Command {
            method,
            params,
            session: session.map(str::to_owned),
        }
    }
}

impl Listener for Visitor<'_> {
    fn heard(&mut self, method: &str, params: &Value, session: Option<&str>) -> Option<Command> {
        match method {
            "Fetch.requestPaused" => return Some(self.serve(params, session)),
            "Page.loadEventFired" => self.loaded_event = true,
            // Refused by the browser's flags, not by us (`browser.rs`): this
            // only says so, as a refused request is said.
            "Network.webSocketCreated" => {
                let url = params["url"].as_str().unwrap_or("a server");
                self.warn(format!(
                    "the page opened a WebSocket to {url}; pages render offline, \
                     so it reached nothing"
                ));
            }
            "Runtime.bindingCalled" if params["name"] == OFFLINE_BINDING => {
                let what = params["payload"].as_str().unwrap_or("network");
                self.warn(format!(
                    "the page opened a {what} connection; pages render offline, \
                     so it reached nothing"
                ));
            }
            "Runtime.exceptionThrown" => {
                let details = &params["exceptionDetails"];
                let what = details["exception"]["description"]
                    .as_str()
                    .or_else(|| details["text"].as_str())
                    .unwrap_or("an error");
                let first = what.lines().next().unwrap_or(what);
                self.warn(format!("the page's script failed: {first}"));
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paused(url: &str) -> Value {
        json!({ "requestId": "r1", "request": { "url": url } })
    }

    #[test]
    fn a_project_file_is_served_and_remembered_by_its_hash() {
        let root = std::env::temp_dir().join(format!("scorsese-visitor-{}", std::process::id()));
        std::fs::create_dir_all(root.join("pages")).unwrap();
        std::fs::write(root.join("pages/a.html"), "hello").unwrap();
        let mut visitor = Visitor::new(&root);
        let reply = visitor
            .heard(
                "Fetch.requestPaused",
                &paused("https://page.scorsese/pages/a.html"),
                Some("s"),
            )
            .unwrap();
        assert_eq!(reply.method, "Fetch.fulfillRequest");
        assert_eq!(reply.params["responseCode"], 200);
        assert_eq!(reply.params["body"], STANDARD.encode("hello"));
        assert_eq!(reply.session.as_deref(), Some("s"));
        assert_eq!(
            visitor.loaded["pages/a.html"].as_deref(),
            Some(scorsese_core::hash_bytes(b"hello").as_str())
        );
        assert!(visitor.warnings.is_empty());

        visitor.heard(
            "Fetch.requestPaused",
            &paused("https://page.scorsese/pages/b.css"),
            None,
        );
        assert_eq!(
            visitor.loaded["pages/b.css"], None,
            "a miss is remembered too"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_internet_is_refused_and_said_once() {
        let mut visitor = Visitor::new(Path::new("/nonexistent"));
        for _ in 0..2 {
            let reply = visitor
                .heard(
                    "Fetch.requestPaused",
                    &paused("https://cdn.example/x.js"),
                    None,
                )
                .unwrap();
            assert_eq!(reply.method, "Fetch.failRequest");
        }
        assert_eq!(visitor.warnings.len(), 1);
        assert!(visitor.warnings[0].contains("https://cdn.example/x.js"));
    }

    #[test]
    fn a_script_error_and_the_load_event_are_heard() {
        let mut visitor = Visitor::new(Path::new("/"));
        let thrown = json!({ "exceptionDetails": { "text": "Uncaught",
            "exception": { "description": "ReferenceError: x is not defined\n    at page" } } });
        assert!(
            visitor
                .heard("Runtime.exceptionThrown", &thrown, None)
                .is_none()
        );
        assert_eq!(
            visitor.warnings,
            ["the page's script failed: ReferenceError: x is not defined"]
        );
        visitor.heard("Page.loadEventFired", &json!({}), None);
        assert!(visitor.loaded_event);
    }

    #[test]
    fn a_websocket_and_a_peer_connection_are_said_once_each() {
        let mut visitor = Visitor::new(Path::new("/"));
        for _ in 0..2 {
            let socket = json!({ "requestId": "w", "url": "wss://192.168.1.8:47773/" });
            assert!(
                visitor
                    .heard("Network.webSocketCreated", &socket, None)
                    .is_none()
            );
            let called = json!({ "name": OFFLINE_BINDING, "payload": "WebRTC" });
            visitor.heard("Runtime.bindingCalled", &called, None);
        }
        let other = json!({ "name": "someone_else", "payload": "x" });
        visitor.heard("Runtime.bindingCalled", &other, None);
        assert_eq!(
            visitor.warnings,
            [
                "the page opened a WebSocket to wss://192.168.1.8:47773/; pages render \
                 offline, so it reached nothing",
                "the page opened a WebRTC connection; pages render offline, so it reached nothing",
            ]
        );
    }
}
