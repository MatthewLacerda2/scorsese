//! Folding a stream's chunks into one [`Reply`].
//!
//! Pure: chunks in, a reply out, and the words handed to a callback on the
//! way. Text arrives a few words a chunk and is joined; a call arrives whole.
//! Thought summaries are progress notes and are not kept — what carries the
//! reasoning is the thought signature, which stays on the part it came on.

use super::{GeminiError, MADE_UP};
use crate::api::gemini::chat::request::{Content, Part as WirePart};
use crate::api::gemini::chat::response::{Chunk, UsageMetadata};
use crate::chat::{Message, Part, Reply, Stop, Streamed};
use crate::prices::chat::Usage;

/// A reply being put together.
#[derive(Debug, Default)]
pub struct Assembler {
    id: String,
    model: String,
    parts: Vec<WirePart>,
    finish: Option<String>,
    blocked: Option<String>,
    usage: UsageMetadata,
    /// Whether the last piece was a thought, so a change of kind ends a block.
    thinking: Option<bool>,
}

impl Assembler {
    /// Take in one chunk.
    pub fn feed(
        &mut self,
        chunk: Chunk,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<(), GeminiError> {
        if let Some(error) = chunk.error {
            return Err(GeminiError::Api {
                status: error.status.unwrap_or_else(|| "UNKNOWN".to_owned()),
                message: error.message,
            });
        }
        self.id = chunk.response_id.unwrap_or(std::mem::take(&mut self.id));
        self.model = chunk
            .model_version
            .unwrap_or(std::mem::take(&mut self.model));
        if let Some(usage) = chunk.usage_metadata {
            self.usage = usage;
        }
        if let Some(reason) = chunk.prompt_feedback.and_then(|f| f.block_reason) {
            self.blocked = Some(reason);
        }
        for candidate in chunk.candidates.into_iter().take(1) {
            if let Some(reason) = candidate.finish_reason {
                self.finish = Some(reason);
            }
            for part in candidate.content.map(|c| c.parts).unwrap_or_default() {
                self.add(part, on);
            }
        }
        Ok(())
    }

    /// One more part.
    fn add(&mut self, part: WirePart, on: &mut dyn FnMut(Streamed<'_>)) {
        let thought = part.thought;
        if self.thinking.is_some_and(|was| was != thought) || part.function_call.is_some() {
            on(Streamed::BlockEnd);
        }
        self.thinking = Some(thought);
        let words = part.text.clone().unwrap_or_default();
        if thought {
            if !words.is_empty() {
                on(Streamed::Progress(&words));
            }
            // The summary is not sent back; a signature it carries is.
            if let Some(signature) = part.thought_signature {
                self.parts.push(WirePart {
                    text: Some(String::new()),
                    thought_signature: Some(signature),
                    ..WirePart::default()
                });
            }
            return;
        }
        if !words.is_empty() {
            on(Streamed::Text(&words));
        }
        let joins = part.function_call.is_none() && part.text.is_some();
        match self.parts.last_mut() {
            Some(last)
                if joins
                    && last.function_call.is_none()
                    && last.thought_signature.is_none()
                    && last.text.is_some() =>
            {
                last.text.get_or_insert_default().push_str(&words);
                last.thought_signature = part.thought_signature;
            }
            _ => self.parts.push(part),
        }
    }

    /// The reply, once the stream has ended.
    pub fn finish(mut self, on: &mut dyn FnMut(Streamed<'_>)) -> Result<Reply, GeminiError> {
        on(Streamed::BlockEnd);
        let message = self.neutral();
        let calls = !message.calls().is_empty();
        let stop = match (self.blocked.take(), self.finish.as_deref()) {
            (Some(reason), _) => refusal(&reason),
            (None, None) => return Err(GeminiError::Cut),
            (None, Some("STOP")) if calls => Stop::ToolUse,
            (None, Some("STOP")) => Stop::EndTurn,
            (None, Some("MAX_TOKENS")) => Stop::MaxTokens,
            (
                None,
                Some(
                    reason @ ("SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII"
                    | "IMAGE_SAFETY"),
                ),
            ) => refusal(reason),
            (None, Some(other)) => Stop::Other(other.to_lowercase()),
        };
        if self.parts.is_empty() {
            self.parts.push(WirePart {
                text: Some(String::new()),
                ..WirePart::default()
            });
        }
        let native = serde_json::value::to_raw_value(&Content {
            role: Some("model".to_owned()),
            parts: self.parts,
        })
        .map_err(|error| GeminiError::Unwritable(error.to_string()))?;
        Ok(Reply {
            model: self.model,
            message,
            native,
            stop,
            thinking: Some(self.usage.thoughts_token_count),
            usage: priced(self.usage),
        })
    }

    /// The parts kept, as the neutral record says them. A call Gemini gave no
    /// id gets one here, unique to the reply, so its result can name it.
    fn neutral(&self) -> Message {
        let reply = self
            .id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let content = self
            .parts
            .iter()
            .enumerate()
            .filter_map(|(index, part)| match (&part.function_call, &part.text) {
                (Some(call), _) => Some(Part::Call {
                    id: call
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("{MADE_UP}{reply}_{index}")),
                    name: call.name.clone(),
                    input: call.args.clone(),
                }),
                (None, Some(text)) if !text.is_empty() => Some(Part::Text { text: text.clone() }),
                _ => None,
            })
            .collect();
        Message::Assistant { content }
    }
}

/// A refusal, by Google's reason.
fn refusal(reason: &str) -> Stop {
    Stop::Refusal {
        category: Some(reason.to_lowercase()),
        explanation: None,
    }
}

/// `usageMetadata` as the rate table prices it: the cached part of the prompt
/// is a cache read, never plain input; thinking is output.
fn priced(usage: UsageMetadata) -> Usage {
    Usage {
        input: usage
            .prompt_token_count
            .saturating_sub(usage.cached_content_token_count)
            .saturating_add(usage.tool_use_prompt_token_count),
        output: usage
            .candidates_token_count
            .saturating_add(usage.thoughts_token_count),
        cache_write_5m: 0,
        cache_write_1h: 0,
        cache_read: usage.cached_content_token_count,
    }
}

/// A whole reply out of a recorded stream — what a fixture test reads.
pub fn replay(
    stream: impl std::io::BufRead,
    on: &mut dyn FnMut(Streamed<'_>),
) -> Result<Reply, GeminiError> {
    let mut assembler = Assembler::default();
    for chunk in crate::api::gemini::chat::chunks(stream) {
        assembler.feed(chunk?, on)?;
    }
    assembler.finish(on)
}
