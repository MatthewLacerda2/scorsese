//! What a streamed `generateContent` sends back: chunks, read permissively.
//!
//! Each `data:` line is a whole `GenerateContentResponse` carrying the parts
//! written since the last one. The finish reason and the full usage arrive on
//! the last; an error mid-stream arrives as an object with only `error`.

use serde::Deserialize;

use super::request::Content;

/// One chunk of the stream.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
    /// The answers; scorsese asks for one.
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    /// The tokens so far — whole on the last chunk.
    #[serde(default)]
    pub usage_metadata: Option<UsageMetadata>,
    /// The model that answered.
    #[serde(default)]
    pub model_version: Option<String>,
    /// The reply's id.
    #[serde(default)]
    pub response_id: Option<String>,
    /// Set when the prompt itself was blocked.
    #[serde(default)]
    pub prompt_feedback: Option<PromptFeedback>,
    /// Set when the API failed partway.
    #[serde(default)]
    pub error: Option<ApiError>,
}

/// One answer.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// More of it.
    #[serde(default)]
    pub content: Option<Content>,
    /// `STOP`, `MAX_TOKENS`, `SAFETY`, … — on the last chunk.
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// `usageMetadata`. Every count is absent when zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    /// The whole prompt, the cached part included.
    #[serde(default)]
    pub prompt_token_count: u64,
    /// The part of the prompt read from the implicit cache.
    #[serde(default)]
    pub cached_content_token_count: u64,
    /// The answer, thinking excluded.
    #[serde(default)]
    pub candidates_token_count: u64,
    /// Thinking, billed as output.
    #[serde(default)]
    pub thoughts_token_count: u64,
    /// Results of Google's own tools, billed as input. scorsese uses none.
    #[serde(default)]
    pub tool_use_prompt_token_count: u64,
}

/// `promptFeedback`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFeedback {
    /// Why the prompt was refused, e.g. `SAFETY`.
    #[serde(default)]
    pub block_reason: Option<String>,
}

/// An error object.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ApiError {
    /// The HTTP status it stands for.
    #[serde(default)]
    pub code: Option<u16>,
    /// In words.
    #[serde(default)]
    pub message: String,
    /// E.g. `UNAVAILABLE`, `RESOURCE_EXHAUSTED`.
    #[serde(default)]
    pub status: Option<String>,
}
