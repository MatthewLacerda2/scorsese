//! The blocks a conversation is made of.
//!
//! Serialise-only, like the request: a reply is read from the stream's events
//! ([`super::stream`]) and assembled into these, never parsed back from JSON —
//! so a tool call's `input` can be kept as the exact text the model wrote
//! ([`RawValue`]) and replayed unchanged, rather than round-tripped through a
//! map that would reorder its keys.

use serde::Serialize;
use serde_json::value::RawValue;

/// One content block.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// Words.
    Text {
        /// The words.
        text: String,
    },
    /// The model's thinking. Under `display: "updates"` its text is a short
    /// progress note or empty; either way it is sent back unchanged, signature
    /// and all, or the model loses its reasoning.
    Thinking {
        /// The note, or nothing.
        thinking: String,
        /// What the API checks the block against when it comes back.
        signature: String,
    },
    /// Thinking the API returned encrypted. Sent back unchanged.
    RedactedThinking {
        /// Opaque.
        data: String,
    },
    /// The model calling a tool.
    ToolUse {
        /// Which call this is; its result names it.
        id: String,
        /// The tool.
        name: String,
        /// The arguments, exactly as the model wrote them.
        input: Box<RawValue>,
    },
    /// What a tool answered, sent back in the next user message.
    ToolResult {
        /// The call it answers.
        tool_use_id: String,
        /// Words and pictures, in the order to read them.
        content: Vec<ResultPart>,
        /// Whether the tool refused rather than answered.
        #[serde(skip_serializing_if = "is_false")]
        is_error: bool,
    },
}

/// A part of a tool's answer.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResultPart {
    /// Words.
    Text {
        /// The words.
        text: String,
    },
    /// A picture.
    Image {
        /// Where its bytes are.
        source: ImageSource,
    },
}

/// A picture's bytes, inline.
#[derive(Debug, Clone, Serialize)]
pub struct ImageSource {
    /// Always `base64`.
    #[serde(rename = "type")]
    pub kind: Base64,
    /// What the bytes are, e.g. `image/png`.
    pub media_type: String,
    /// The bytes, base64-encoded.
    pub data: String,
}

/// The one image source kind scorsese sends.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Base64 {
    /// Inline bytes.
    Base64,
}

impl ResultPart {
    /// A PNG, already base64-encoded.
    pub fn png(data: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource {
                kind: Base64::Base64,
                media_type: "image/png".to_owned(),
                data: data.into(),
            },
        }
    }
}

/// For `skip_serializing_if`: `is_error` is sent only when it is true.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde hands a reference.
fn is_false(value: &bool) -> bool {
    !*value
}
