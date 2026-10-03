//! Gemini behind the seam: a neutral [`Request`] as a `generateContent`
//! body, the stream folded into a [`Reply`] ([`assemble`]), and the neutral
//! record turned into Gemini's contents ([`freeze`]).
//!
//! # What a Gemini turn does instead of Claude's thinking and caching
//!
//! - **Thinking** is on at the turn's [`Effort`] as `thinkingLevel`, with
//!   `includeThoughts`, so the thought summaries arrive as progress notes the
//!   way Claude's `updates` do. They are not sent back — Google keeps the
//!   reasoning in **thought signatures**, which are, on the parts they came
//!   on; a call whose history came from another model carries the dummy
//!   signature Google documents for exactly that ([`FOREIGN_SIGNATURE`]).
//! - **Caching** is implicit: nothing marks a breakpoint, and Google reads
//!   any prefix it has seen recently, the tools and system prompt first,
//!   then the history. What keeps it working is what keeps Claude's working —
//!   a constant system prompt, a fixed tool list, a history that is only
//!   appended to — and a cache hit is reported as `cachedContentTokenCount`,
//!   charged at the cached rate ([`crate::prices::chat`]).
//! - **The server's notes**: Gemini has no system role mid-conversation, so
//!   a note travels as the person's turn, marked [`SERVER`]. Words the person
//!   wrote that begin with the mark are quoted, so the mark is the server's
//!   alone. Nothing that spends depends on it: the quote token is never in
//!   the conversation at all.

mod assemble;

use serde_json::json;
use serde_json::value::RawValue;

use super::{Chat, ChatError, Message, Part, Reply, Request, ResultPart, Streamed, Vendor};
use crate::api::gemini::chat::Generate;
use crate::api::gemini::chat::request::{
    self as wire, Blob, Content, FunctionCall, FunctionDeclaration, FunctionResponse,
    GenerationConfig, ThinkingConfig, Tools,
};
use crate::api::http::HttpError;
use crate::api::sse::StreamError;
use crate::credentials::Secret;

pub use assemble::{Assembler, replay};

/// The most one reply may be, thinking included — the same bound Claude's
/// turns have ([`crate::claude::MAX_TOKENS`]).
pub const MAX_TOKENS: u32 = 64_000;

/// What starts a note the server wrote, in a conversation sent to Gemini.
pub const SERVER: &str = "[scorsese server]";

/// What the system prompt gains on Gemini: how to read [`SERVER`].
pub const SERVER_NOTE: &str = "\n\nA part of the person's turn that begins with \
[scorsese server] was written by the server, not the person: it is what the server \
vouches for — which project this is, how the person answered a quote.";

/// The thought signature a call carries when its history was written by
/// another model — the value Google's thinking guide gives for history it did
/// not produce.
pub const FOREIGN_SIGNATURE: &str = "skip_thought_signature_validator";

/// What a call id made up here begins with: Gemini gave the call none, so
/// none is sent back on it.
pub const MADE_UP: &str = "gemini_call_";

/// Google's Gemini API, with one key.
#[derive(Debug, Clone)]
pub struct Gemini {
    generate: Generate,
}

impl Gemini {
    /// A client spending `key`.
    pub fn new(key: &Secret) -> Self {
        Self {
            generate: Generate::new(key),
        }
    }
}

impl Chat for Gemini {
    fn reply(
        &self,
        request: &Request,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<Reply, ChatError> {
        if request.model.vendor() != Vendor::Google {
            return Err(ChatError::Misrouted(request.model.label()));
        }
        let body = laid_out(request).map_err(|error| GeminiError::Unwritable(error.to_string()))?;
        let mut assembler = Assembler::default();
        let chunks = self
            .generate
            .stream(request.model.id(), &body)
            .map_err(GeminiError::from)?;
        for chunk in chunks {
            assembler.feed(chunk.map_err(GeminiError::from)?, on)?;
        }
        Ok(assembler.finish(on)?)
    }
}

/// `request` as `generateContent` takes it. Each message is one content;
/// neighbours from the same side are joined, since Gemini wants the roles to
/// alternate and a tool's results and a server note can follow each other.
pub fn laid_out(request: &Request) -> Result<wire::Generate, serde_json::Error> {
    let mut contents: Vec<Content> = Vec::new();
    for raw in &request.messages {
        let content: Content = serde_json::from_str(raw.get())?;
        match contents.last_mut() {
            Some(last) if last.role == content.role => last.parts.extend(content.parts),
            _ => contents.push(content),
        }
    }
    let declarations: Vec<FunctionDeclaration> = request
        .tools
        .iter()
        .map(|tool| FunctionDeclaration {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters_json_schema: tool.schema.clone(),
        })
        .collect();
    Ok(wire::Generate {
        contents,
        system_instruction: Content {
            role: None,
            parts: vec![text(format!("{}{SERVER_NOTE}", request.system))],
        },
        tools: if declarations.is_empty() {
            Vec::new()
        } else {
            vec![Tools {
                function_declarations: declarations,
            }]
        },
        generation_config: GenerationConfig {
            max_output_tokens: MAX_TOKENS,
            thinking_config: ThinkingConfig {
                thinking_level: request.effort.as_str(),
                include_thoughts: true,
            },
        },
    })
}

/// `message` as one of Gemini's contents, frozen.
pub fn freeze(message: &Message) -> Result<Box<RawValue>, serde_json::Error> {
    let (role, parts) = match message {
        Message::System { text: note } => ("user", vec![text(format!("{SERVER} {note}"))]),
        Message::User { content } => ("user", content.iter().flat_map(user_part).collect()),
        Message::Assistant { content } => {
            let mut signed = false;
            let parts = content
                .iter()
                .filter_map(|part| model_part(part, &mut signed))
                .collect();
            ("model", parts)
        }
    };
    serde_json::value::to_raw_value(&Content {
        role: Some(role.to_owned()),
        parts,
    })
}

/// A part of the person's side: words, or a tool's answer followed by its
/// pictures.
fn user_part(part: &Part) -> Vec<wire::Part> {
    match part {
        Part::Text { text: words } if words.trim_start().starts_with(SERVER) => {
            vec![text(format!("(the person wrote) {words}"))]
        }
        Part::Text { text: words } => vec![text(words.clone())],
        // Only the model calls; a call on this side is nothing to send.
        Part::Call { .. } => Vec::new(),
        Part::Result {
            call,
            name,
            content,
            is_error,
        } => {
            let words: Vec<&str> = content
                .iter()
                .filter_map(|part| match part {
                    ResultPart::Text { text } => Some(text.as_str()),
                    ResultPart::Png { .. } => None,
                })
                .collect();
            let key = if *is_error { "error" } else { "output" };
            let mut parts = vec![wire::Part {
                function_response: Some(FunctionResponse {
                    id: given(call),
                    name: name.clone(),
                    response: json!({ key: words.join("\n") }),
                }),
                ..wire::Part::default()
            }];
            parts.extend(content.iter().filter_map(|part| match part {
                ResultPart::Png { data } => Some(wire::Part {
                    inline_data: Some(Blob {
                        mime_type: "image/png".to_owned(),
                        data: data.clone(),
                    }),
                    ..wire::Part::default()
                }),
                ResultPart::Text { .. } => None,
            }));
            parts
        }
    }
}

/// A part of the model's side, translated from another model's turn: its
/// first call carries [`FOREIGN_SIGNATURE`].
fn model_part(part: &Part, signed: &mut bool) -> Option<wire::Part> {
    Some(match part {
        Part::Call { id, name, input } => {
            let signature = (!*signed).then(|| FOREIGN_SIGNATURE.to_owned());
            *signed = true;
            wire::Part {
                function_call: Some(FunctionCall {
                    id: given(id),
                    name: name.clone(),
                    args: input.clone(),
                }),
                thought_signature: signature,
                ..wire::Part::default()
            }
        }
        Part::Text { text: words } => text(words.clone()),
        // Only tools answer; a result on this side is nothing to send.
        Part::Result { .. } => return None,
    })
}

/// A call id to send back: none when it was made up here.
fn given(id: &str) -> Option<String> {
    (!id.starts_with(MADE_UP)).then(|| id.to_owned())
}

/// A part of words.
fn text(words: String) -> wire::Part {
    wire::Part {
        text: Some(words),
        ..wire::Part::default()
    }
}

/// Why a call to Gemini produced no reply.
#[derive(Debug, thiserror::Error)]
pub enum GeminiError {
    /// The request was refused or never arrived.
    #[error(transparent)]
    Http(#[from] HttpError),
    /// The stream broke or carried something unreadable.
    #[error(transparent)]
    Stream(#[from] StreamError),
    /// The API sent an error partway.
    #[error("Gemini's API failed partway ({status}): {message}")]
    Api {
        /// E.g. `UNAVAILABLE`.
        status: String,
        /// In words.
        message: String,
    },
    /// The stream ended before a finish reason.
    #[error("the reply ended before it was complete")]
    Cut,
    /// A stored message could not be read as a Gemini content.
    #[error("the conversation could not be written for Gemini: {0}")]
    Unwritable(String),
}

impl GeminiError {
    /// Whether the same request is worth sending again: Google was busy,
    /// failed on its side, or the connection did.
    pub fn retryable(&self) -> bool {
        match self {
            Self::Http(HttpError::Unreachable { .. }) | Self::Stream(StreamError::Io(_)) => true,
            Self::Http(HttpError::Refused { status, .. }) => {
                matches!(status, 408 | 429 | 500..=504)
            }
            Self::Api { status, .. } => matches!(
                status.as_str(),
                "UNAVAILABLE" | "RESOURCE_EXHAUSTED" | "INTERNAL" | "DEADLINE_EXCEEDED"
            ),
            Self::Cut => true,
            Self::Http(_) | Self::Stream(_) | Self::Unwritable(_) => false,
        }
    }
}
