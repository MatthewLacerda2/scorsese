//! The assistant's vendor-neutral seam (`src/chat`): Gemini's stream folded
//! into a reply, the neutral record written in each vendor's wire, and a
//! history replayed across a change of model — never a real call.
//!
//! The Gemini fixtures under `fixtures/gemini/chat-*.sse` were written from
//! Google's `generateContent` reference (2026-10-03), not captured.

mod anthropic;
mod edges;
mod gemini;
mod seam;
mod thoughts;

use scorsese_providers::chat::gemini::{GeminiError, replay as fold};
use scorsese_providers::chat::{Reply, Streamed};

/// A recorded Gemini stream replayed: the reply, and everything streamed on
/// the way.
fn replay(name: &str) -> (Result<Reply, GeminiError>, Vec<String>) {
    let path = format!("{}/fixtures/gemini/{name}", env!("CARGO_MANIFEST_DIR"));
    let stream = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    let mut heard = Vec::new();
    let reply = fold(stream.as_bytes(), &mut |piece| {
        heard.push(match piece {
            Streamed::Text(text) => format!("text:{text}"),
            Streamed::Progress(text) => format!("progress:{text}"),
            Streamed::BlockEnd => "end".to_owned(),
        });
    });
    (reply, heard)
}
