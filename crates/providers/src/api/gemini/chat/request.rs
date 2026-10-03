//! What `generateContent` is sent, declared field for field as Google names
//! it: camel case, unlike the Interactions endpoint's snake case.
//!
//! [`Content`] and [`Part`] are read as well as written: a reply streams back
//! as the same contents, and the model's turn is sent back as it came.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One call to `streamGenerateContent`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Generate {
    /// The conversation, oldest first; roles alternate `user` and `model`.
    pub contents: Vec<Content>,
    /// The instructions, as a content with no role.
    pub system_instruction: Content,
    /// What the model may call. Omitted when there is nothing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tools>,
    /// Length and thinking.
    pub generation_config: GenerationConfig,
}

/// One turn of the conversation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Content {
    /// `user` or `model`; absent on the system instruction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// What it holds, in order.
    #[serde(default)]
    pub parts: Vec<Part>,
}

/// One piece of a turn. Exactly one of its payload fields is set; the
/// thinking fields ride along with any of them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    /// Words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Whether the words are a summary of the model's thinking, not its answer.
    #[serde(default, skip_serializing_if = "is_false")]
    pub thought: bool,
    /// Opaque: the model's reasoning state, which must come back on the part
    /// it arrived on for the model to carry its reasoning across a tool call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
    /// The model calling a tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    /// A tool's answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
    /// Bytes inline — a picture a tool answered with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<Blob>,
}

/// `functionCall`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The call's id, when the API gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The tool.
    pub name: String,
    /// Its arguments, an object.
    #[serde(default = "empty")]
    pub args: Value,
}

/// `functionResponse`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// The call it answers, when the call had an id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The tool that answered.
    pub name: String,
    /// What it said: `{"output": …}`, or `{"error": …}` for a refusal — the
    /// keys Google's guide uses.
    pub response: Value,
}

/// `inlineData`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    /// E.g. `image/png`.
    pub mime_type: String,
    /// The bytes, base64.
    pub data: String,
}

/// `tools[]`: one entry holding every function.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    /// The functions.
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// One function the model may call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDeclaration {
    /// What the model calls it by.
    pub name: String,
    /// What it does.
    pub description: String,
    /// Its arguments as full JSON Schema — `parametersJsonSchema` rather than
    /// `parameters`, which takes only an OpenAPI subset the registry's schemas
    /// are not written in.
    pub parameters_json_schema: Value,
}

/// `generationConfig`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// The most the reply may be, thinking included.
    pub max_output_tokens: u32,
    /// How it thinks.
    pub thinking_config: ThinkingConfig,
}

/// `thinkingConfig`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    /// `low`, `medium` or `high` — every level both offered models accept.
    pub thinking_level: &'static str,
    /// Whether thought summaries come back: they are the progress notes.
    pub include_thoughts: bool,
}

/// An empty object: what a call with no arguments carries.
fn empty() -> Value {
    Value::Object(serde_json::Map::new())
}

/// For `skip_serializing_if`: `thought` is sent only when it is true.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde hands a reference.
fn is_false(value: &bool) -> bool {
    !*value
}
