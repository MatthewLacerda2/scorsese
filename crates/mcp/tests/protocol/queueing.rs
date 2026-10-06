//! `jobs`, following and stopping renders across a live session (#700): the client
//! writes a line, reads the answer, and writes the next — over real pipes, so
//! a background render runs on while the session waits between questions.

use std::io::{BufRead, BufReader, Write};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::fixture::project;
use crate::said;

/// A server on the far side of two pipes.
struct Live {
    to: Option<std::io::PipeWriter>,
    from: BufReader<std::io::PipeReader>,
    server: Option<JoinHandle<()>>,
    id: u64,
}

impl Live {
    fn start() -> Self {
        let (input, to) = std::io::pipe().expect("a pipe");
        let (from, output) = std::io::pipe().expect("a pipe");
        let server = std::thread::spawn(move || {
            scorsese_mcp::serve(BufReader::new(input), output).expect("the server runs");
        });
        Self {
            to: Some(to),
            from: BufReader::new(from),
            server: Some(server),
            id: 0,
        }
    }

    /// Calls `name` and returns what it said, and whether it refused.
    fn ask(&mut self, name: &str, arguments: Value) -> (String, bool) {
        self.id += 1;
        let frame = json!({
            "jsonrpc": "2.0", "id": self.id, "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        let to = self.to.as_mut().expect("open until dropped");
        writeln!(to, "{frame}").expect("the server reads");
        let mut line = String::new();
        self.from.read_line(&mut line).expect("the server answers");
        let reply: Value = serde_json::from_str(&line).expect("an answer is JSON");
        assert_eq!(reply["id"], json!(self.id), "{reply}");
        said(&reply)
    }

    /// Asks `jobs` about `job` until its line stops saying running.
    fn settled(&mut self, project: &Value, job: u64) -> String {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let (text, failed) = self.ask("jobs", json!({ "project": project, "job": job }));
            assert!(!failed, "{text}");
            if !text.contains(": running") || Instant::now() > deadline {
                return text;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        drop(self.to.take());
        if let Some(server) = self.server.take() {
            server.join().ok();
        }
    }
}

#[test]
fn a_background_render_is_followed_to_the_file_it_wrote() {
    let dir = project("jobs-done");
    let project = json!(dir);
    let mut live = Live::start();
    let (text, _) = live.ask("jobs", json!({ "project": project }));
    assert!(text.starts_with("No renders of this project yet"), "{text}");
    let arguments =
        json!({ "project": project, "out": "cut.mp4", "resolution": "160x90", "range": "0:20" });
    let (text, failed) = live.ask("render", arguments);
    assert!(
        !failed && text.starts_with("Rendering cut.mp4 as job 1"),
        "{text}"
    );

    let (text, _) = live.ask("jobs", json!({ "project": project }));
    assert!(text.starts_with("job 1 (render): "), "{text}");
    let text = live.settled(&project, 1);
    assert!(
        text.starts_with("job 1 (render): done — wrote cut.mp4"),
        "{text}"
    );
    assert!(dir.join("cut.mp4").is_file());

    let (text, failed) = live.ask("jobs", json!({ "project": project, "cancel": 1 }));
    assert!(
        !failed && text.contains("done — wrote cut.mp4"),
        "left as it is: {text}"
    );
    let (text, failed) = live.ask("jobs", json!({ "project": project, "job": 2 }));
    assert!(failed && text.contains("there is no job 2"), "{text}");
    drop(live);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn cancelling_through_jobs_stops_a_render_and_keeps_no_file() {
    let dir = project("jobs-cancelled");
    let project = json!(dir);
    let mut live = Live::start();
    let arguments = json!({ "project": project, "out": "cut.mp4", "resolution": "160x90" });
    let (text, failed) = live.ask("render", arguments);
    assert!(!failed, "{text}");
    let (text, failed) = live.ask("jobs", json!({ "project": project, "cancel": 1 }));
    assert!(!failed && text.starts_with("Stopping job 1"), "{text}");
    let text = live.settled(&project, 1);
    assert!(text.starts_with("job 1 (render): cancelled — "), "{text}");
    assert!(
        !dir.join("cut.mp4").exists(),
        "the unfinished file was left"
    );
    drop(live);
    std::fs::remove_dir_all(dir).ok();
}
