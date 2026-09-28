//! Claude's wire (`src/api/anthropic`) and the client over it (`src/claude`),
//! against recorded replies — never a real call.
//!
//! The fixtures under `fixtures/anthropic/` are streams in the shapes
//! Anthropic's reference documents (the `claude-api` skill, 2026-09-28: the
//! raw-HTTP streaming example, the prompt-caching page's `usage` fields, and
//! the Claude Opus 5.5 notes on `thinking_delta` progress updates and
//! `stop_details`). They were written from that reference rather than captured,
//! because no key existed when this landed; #567's live provider check is what
//! replays a real call against them.

mod requests;
mod streams;

use scorsese_providers::claude::{self, Response, Streamed};

/// A recorded stream, by its file name.
fn fixture(name: &str) -> String {
    let path = format!("{}/fixtures/anthropic/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// A recorded stream replayed: the reply, and everything streamed on the way.
fn replay(name: &str) -> (Result<Response, claude::ClaudeError>, Vec<String>) {
    let mut heard = Vec::new();
    let reply = claude::replay(fixture(name).as_bytes(), &mut |piece| {
        heard.push(match piece {
            Streamed::Text(text) => format!("text:{text}"),
            Streamed::Progress(text) => format!("progress:{text}"),
            Streamed::BlockEnd => "end".to_owned(),
        });
    });
    (reply, heard)
}
