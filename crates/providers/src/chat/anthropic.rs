//! Claude behind the seam: a neutral [`Request`] laid out the way
//! [`crate::claude`] caches it, a reply read back as a neutral [`Message`],
//! and the neutral record turned into Anthropic's blocks and back.

use std::collections::HashMap;

use serde_json::Value;
use serde_json::value::RawValue;

use super::{Chat, ChatError, Effort, Message, Part, Reply, Request, ResultPart, Streamed, Vendor};
use crate::api::anthropic::content::{self, Block};
use crate::api::anthropic::request::{self as wire, MessageContent, Role};
use crate::claude::{self, Anthropic, Claude, MAX_TOKENS, Response, Settings};

impl Chat for Anthropic {
    fn reply(
        &self,
        request: &Request,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<Reply, ChatError> {
        if request.model.vendor() != Vendor::Anthropic {
            return Err(ChatError::Misrouted(request.model.label()));
        }
        let response = Claude::reply(self, &laid_out(request), on)?;
        Ok(reply_of(response)?)
    }
}

/// `request` as the Messages API takes it, cache breakpoints and all.
pub fn laid_out(request: &Request) -> wire::Request {
    let settings = Settings {
        model: request.model.id().to_owned(),
        effort: match request.effort {
            Effort::Low => wire::Effort::Low,
            Effort::Medium => wire::Effort::Medium,
            Effort::High => wire::Effort::High,
        },
        max_tokens: MAX_TOKENS,
    };
    let tools = request
        .tools
        .iter()
        .map(|tool| wire::Tool {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.schema.clone(),
        })
        .collect();
    claude::request(&settings, &request.system, tools, request.messages.clone())
}

/// A whole Claude reply as the seam hands it on: thinking stays in the bytes
/// and out of the neutral message.
pub fn reply_of(response: Response) -> Result<Reply, claude::ClaudeError> {
    let native = wire::Message {
        role: Role::Assistant,
        content: MessageContent::Blocks(response.content.clone()),
    }
    .raw()
    .map_err(|error| claude::ClaudeError::Arguments(error.to_string()))?;
    let content = response
        .content
        .iter()
        .filter_map(|block| match block {
            Block::Text { text } => Some(Part::Text { text: text.clone() }),
            Block::ToolUse { id, name, input } => Some(Part::Call {
                id: id.clone(),
                name: name.clone(),
                // Checked as JSON when the block was assembled.
                input: serde_json::from_str(input.get()).unwrap_or(Value::Null),
            }),
            _ => None,
        })
        .collect();
    Ok(Reply {
        model: response.model,
        message: Message::Assistant { content },
        native,
        stop: response.stop,
        usage: response.usage,
        thinking: None,
    })
}

/// `message` as Anthropic's blocks, frozen.
pub fn freeze(message: &Message) -> Result<Box<RawValue>, serde_json::Error> {
    let (role, content) = match message {
        Message::System { text } => (Role::System, MessageContent::Text(text.clone())),
        Message::User { content } => (Role::User, MessageContent::Blocks(blocks(content)?)),
        Message::Assistant { content } => {
            (Role::Assistant, MessageContent::Blocks(blocks(content)?))
        }
    };
    wire::Message { role, content }.raw()
}

/// Neutral parts as content blocks.
fn blocks(parts: &[Part]) -> Result<Vec<Block>, serde_json::Error> {
    parts
        .iter()
        .map(|part| {
            Ok(match part {
                Part::Text { text } => Block::Text { text: text.clone() },
                Part::Call { id, name, input } => Block::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: serde_json::value::to_raw_value(input)?,
                },
                Part::Result {
                    call,
                    content,
                    is_error,
                    ..
                } => Block::ToolResult {
                    tool_use_id: call.clone(),
                    content: content
                        .iter()
                        .map(|part| match part {
                            ResultPart::Text { text } => {
                                content::ResultPart::Text { text: text.clone() }
                            }
                            ResultPart::Png { data } => content::ResultPart::png(data.clone()),
                        })
                        .collect(),
                    is_error: *is_error,
                },
            })
        })
        .collect()
}

/// Messages kept as Anthropic's JSON alone — every turn from before #705 —
/// read back as the neutral record. `names` carries each call's tool from
/// one turn to the next, since a result names only its call's id.
///
/// What it does not recognise (a thinking block) it leaves out: the record
/// says what was said, and Claude's reasoning is not that.
pub fn read(messages: &[Box<RawValue>], names: &mut HashMap<String, String>) -> Vec<Message> {
    messages
        .iter()
        .filter_map(|raw| serde_json::from_str::<Value>(raw.get()).ok())
        .map(|message| {
            let role = message.get("role").and_then(Value::as_str).unwrap_or("");
            let content = message.get("content").cloned().unwrap_or(Value::Null);
            if let Value::String(text) = &content {
                return match role {
                    "system" => Message::System { text: text.clone() },
                    "assistant" => Message::assistant(text.clone()),
                    _ => Message::user(text.clone()),
                };
            }
            let parts = content
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|block| part(block, names))
                .collect();
            match role {
                "assistant" => Message::Assistant { content: parts },
                _ => Message::User { content: parts },
            }
        })
        .collect()
}

/// One stored block as a neutral part, if it is one the record keeps.
fn part(block: &Value, names: &mut HashMap<String, String>) -> Option<Part> {
    let text = |value: &Value, field: &str| {
        value
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    match block.get("type")?.as_str()? {
        "text" => Some(Part::Text {
            text: text(block, "text"),
        }),
        "tool_use" => {
            let (id, name) = (text(block, "id"), text(block, "name"));
            names.insert(id.clone(), name.clone());
            let input = block.get("input").cloned().unwrap_or(Value::Null);
            Some(Part::Call { id, name, input })
        }
        "tool_result" => {
            let call = text(block, "tool_use_id");
            let content = block
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|piece| match piece.get("type")?.as_str()? {
                    "text" => Some(ResultPart::Text {
                        text: text(piece, "text"),
                    }),
                    "image" => Some(ResultPart::Png {
                        data: text(piece.get("source")?, "data"),
                    }),
                    _ => None,
                })
                .collect();
            Some(Part::Result {
                name: names.get(&call).cloned().unwrap_or_default(),
                call,
                content,
                is_error: block.get("is_error").and_then(Value::as_bool) == Some(true),
            })
        }
        _ => None,
    }
}
