//! Folding a stream's events into one [`Response`].
//!
//! Pure: events in, a reply out, and the text handed to a callback on the way.
//! A tool call's arguments arrive as fragments of JSON text; they are joined
//! and kept **as that text** — the exact bytes the model wrote are what is
//! sent back next call — and only checked to be JSON.

use serde_json::value::RawValue;

use super::{ClaudeError, Response, Stop, Streamed};
use crate::api::anthropic::content::Block;
use crate::api::anthropic::stream::{BlockStart, Delta, Ending, Event, UsageFields};
use crate::prices::claude::Usage;

/// A reply being put together.
#[derive(Debug, Default)]
pub struct Assembler {
    id: String,
    model: String,
    usage: UsageFields,
    blocks: Vec<Option<Partial>>,
    ending: Option<Ending>,
}

/// A block being put together.
#[derive(Debug)]
enum Partial {
    Text(String),
    Thinking {
        text: String,
        signature: String,
    },
    Redacted(String),
    ToolUse {
        id: String,
        name: String,
        json: String,
    },
}

impl Assembler {
    /// Take in one event; the whole reply once it is complete.
    pub fn feed(
        &mut self,
        event: Event,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<Option<Response>, ClaudeError> {
        match event {
            Event::MessageStart { message } => {
                self.id = message.id;
                self.model = message.model;
                merge(&mut self.usage, message.usage);
            }
            Event::ContentBlockStart {
                index,
                content_block,
            } => self.start(index, content_block),
            Event::ContentBlockDelta { index, delta } => self.add(index, delta, on),
            Event::ContentBlockStop { .. } => on(Streamed::BlockEnd),
            Event::MessageDelta { delta, usage } => {
                self.ending = Some(delta);
                if let Some(usage) = usage {
                    merge(&mut self.usage, usage);
                }
            }
            Event::MessageStop => return self.finish().map(Some),
            Event::Error { error } => {
                return Err(ClaudeError::Api {
                    kind: error.kind,
                    message: error.message,
                });
            }
            Event::Ping | Event::Other => {}
        }
        Ok(None)
    }

    /// A block begins at `index`. One this client does not know is left out.
    fn start(&mut self, index: usize, block: BlockStart) {
        if self.blocks.len() <= index {
            self.blocks.resize_with(index + 1, || None);
        }
        self.blocks[index] = match block {
            BlockStart::Text { text } => Some(Partial::Text(text)),
            BlockStart::Thinking {
                thinking,
                signature,
            } => Some(Partial::Thinking {
                text: thinking,
                signature,
            }),
            BlockStart::RedactedThinking { data } => Some(Partial::Redacted(data)),
            BlockStart::ToolUse { id, name } => Some(Partial::ToolUse {
                id,
                name,
                json: String::new(),
            }),
            BlockStart::Other => None,
        };
    }

    /// More of the block at `index`.
    fn add(&mut self, index: usize, delta: Delta, on: &mut dyn FnMut(Streamed<'_>)) {
        let Some(Some(block)) = self.blocks.get_mut(index) else {
            return;
        };
        match (block, delta) {
            (Partial::Text(text), Delta::TextDelta { text: more }) => {
                on(Streamed::Text(&more));
                text.push_str(&more);
            }
            (Partial::Thinking { text, .. }, Delta::ThinkingDelta { thinking }) => {
                if !thinking.is_empty() {
                    on(Streamed::Progress(&thinking));
                }
                text.push_str(&thinking);
            }
            (Partial::Thinking { signature, .. }, Delta::SignatureDelta { signature: more }) => {
                signature.push_str(&more);
            }
            (Partial::ToolUse { json, .. }, Delta::InputJsonDelta { partial_json }) => {
                json.push_str(&partial_json);
            }
            _ => {}
        }
    }

    /// The reply, complete.
    fn finish(&mut self) -> Result<Response, ClaudeError> {
        let content = std::mem::take(&mut self.blocks)
            .into_iter()
            .flatten()
            .map(Partial::into_block)
            .collect::<Result<Vec<_>, _>>()?;
        let ending = self.ending.take();
        let stop = match ending
            .as_ref()
            .and_then(|ending| ending.stop_reason.as_deref())
        {
            Some("end_turn" | "stop_sequence") => Stop::EndTurn,
            Some("tool_use") => Stop::ToolUse,
            Some("max_tokens") => Stop::MaxTokens,
            Some("refusal") => {
                let details = ending.and_then(|ending| ending.stop_details);
                Stop::Refusal {
                    category: details.as_ref().and_then(|d| d.category.clone()),
                    explanation: details.and_then(|d| d.explanation),
                }
            }
            Some(other) => Stop::Other(other.to_owned()),
            None => Stop::Other("unknown".to_owned()),
        };
        Ok(Response {
            id: std::mem::take(&mut self.id),
            model: std::mem::take(&mut self.model),
            content,
            stop,
            usage: priced(&self.usage),
        })
    }
}

impl Partial {
    /// The finished block.
    fn into_block(self) -> Result<Block, ClaudeError> {
        Ok(match self {
            Self::Text(text) => Block::Text { text },
            Self::Thinking { text, signature } => Block::Thinking {
                thinking: text,
                signature,
            },
            Self::Redacted(data) => Block::RedactedThinking { data },
            Self::ToolUse { id, name, json } => {
                // A call with no arguments streams no fragments at all.
                let json = if json.trim().is_empty() {
                    "{}".to_owned()
                } else {
                    json
                };
                let input = RawValue::from_string(json)
                    .map_err(|error| ClaudeError::Arguments(error.to_string()))?;
                Block::ToolUse { id, name, input }
            }
        })
    }
}

/// `later`'s fields over `usage`'s, where `later` has them.
fn merge(usage: &mut UsageFields, later: UsageFields) {
    let keep = |old: &mut Option<u64>, new: Option<u64>| {
        if new.is_some() {
            *old = new;
        }
    };
    keep(&mut usage.input_tokens, later.input_tokens);
    keep(&mut usage.output_tokens, later.output_tokens);
    keep(
        &mut usage.cache_creation_input_tokens,
        later.cache_creation_input_tokens,
    );
    keep(
        &mut usage.cache_read_input_tokens,
        later.cache_read_input_tokens,
    );
    if later.cache_creation.is_some() {
        usage.cache_creation = later.cache_creation;
    }
}

/// The usage as the rate table prices it. Cache writes are split by lifetime
/// when the API says how; a bare total is counted as five-minute writes, the
/// cheaper of the two, since that is the default lifetime.
fn priced(usage: &UsageFields) -> Usage {
    let (five_minutes, one_hour) = match usage.cache_creation {
        Some(split) => (
            split.ephemeral_5m_input_tokens,
            split.ephemeral_1h_input_tokens,
        ),
        None => (usage.cache_creation_input_tokens.unwrap_or(0), 0),
    };
    Usage {
        input: usage.input_tokens.unwrap_or(0),
        output: usage.output_tokens.unwrap_or(0),
        cache_write_5m: five_minutes,
        cache_write_1h: one_hour,
        cache_read: usage.cache_read_input_tokens.unwrap_or(0),
    }
}
