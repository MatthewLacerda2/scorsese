//! The conversation as no vendor in particular writes it.
//!
//! Every turn is kept twice (#705): once as the exact bytes the model it ran
//! on was sent — the vendor's wire, which is what its prompt cache and, on
//! Claude, its thinking blocks are valid against — and once as a [`Message`]
//! here, which says only what was said: words, tool calls, tool results and
//! the server's notes. A turn on another model is built from this record, so
//! a project can change model mid-conversation and the history comes along.
//!
//! What does not come along is what only one vendor understands: Claude's
//! thinking blocks and signatures, Gemini's thought signatures. Those live in
//! the exact bytes, and a turn on the same model replays them.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One message of the conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Message {
    /// The person — and the results of the tools the model called, which
    /// every vendor sends back on the person's side.
    User {
        /// What it says.
        content: Vec<Part>,
    },
    /// The model.
    Assistant {
        /// What it says, and the tools it calls.
        content: Vec<Part>,
    },
    /// The server, mid-conversation: a fact it vouches for (which project
    /// this is, how the person answered a quote). Neither the person nor a
    /// tool can write one.
    System {
        /// The words.
        text: String,
    },
}

/// A piece of a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    /// Words.
    Text {
        /// The words.
        text: String,
    },
    /// The model calling a tool.
    Call {
        /// Which call this is; its result names it.
        id: String,
        /// The tool.
        name: String,
        /// Its arguments.
        input: Value,
    },
    /// What a tool answered.
    Result {
        /// The call it answers.
        call: String,
        /// The tool that answered — Gemini names a result by its tool.
        name: String,
        /// Words and pictures, in the order to read them.
        content: Vec<ResultPart>,
        /// Whether the tool refused rather than answered.
        #[serde(default)]
        is_error: bool,
    },
}

/// A piece of a tool's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResultPart {
    /// Words.
    Text {
        /// The words.
        text: String,
    },
    /// A PNG, base64-encoded.
    Png {
        /// The bytes, base64.
        data: String,
    },
}

/// A tool call out of a reply, for running it.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The call's id, which its result names.
    pub id: String,
    /// The tool.
    pub name: String,
    /// Its arguments.
    pub input: Value,
}

impl Message {
    /// A person's words.
    pub fn user(text: impl Into<String>) -> Self {
        Self::User {
            content: vec![Part::Text { text: text.into() }],
        }
    }

    /// The model's words.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::Assistant {
            content: vec![Part::Text { text: text.into() }],
        }
    }

    /// Its parts; none for a system note.
    pub fn parts(&self) -> &[Part] {
        match self {
            Self::User { content } | Self::Assistant { content } => content,
            Self::System { .. } => &[],
        }
    }

    /// Its words, joined: what the model said, when it is the model's.
    pub fn text(&self) -> String {
        let texts: Vec<&str> = self
            .parts()
            .iter()
            .filter_map(|part| match part {
                Part::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .filter(|text| !text.trim().is_empty())
            .collect();
        texts.join("\n\n")
    }

    /// The tool calls it makes, in order — none unless it is the model's.
    pub fn calls(&self) -> Vec<Call> {
        let Self::Assistant { content } = self else {
            return Vec::new();
        };
        content
            .iter()
            .filter_map(|part| match part {
                Part::Call { id, name, input } => Some(Call {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                }),
                _ => None,
            })
            .collect()
    }
}
