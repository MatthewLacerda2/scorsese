//! The tools, and the one rule about them.
//!
//! **Every tool carries a description, and every argument carries one too.**
//! A description is not a courtesy: it is the entire interface a client has to
//! a tool. An undescribed tool is a capability that exists and cannot be
//! found — nothing fails, the assistant on the other end simply never calls
//! it. `tests/described.rs` walks this registry and fails on one that is
//! missing, from the first tool onwards.
//!
//! Nothing here holds session state. Every tool takes the project directory it
//! works on, reads it, does one thing, and returns. That makes each call
//! independent of every other, which is what lets a client crash, reconnect,
//! or run two conversations against one project without a server-side notion
//! of "the open project" going stale behind its back.

mod create;
mod edit;
mod inspect;
mod level;
mod script;
mod still;
mod synth;

use serde_json::Value;

use crate::base64;

/// The `mimeType` an image block carries. One kind, because there is one kind
/// of picture this server produces — see [`scorsese_render::frames`] for why it
/// is PNG and not something smaller.
const PNG: &str = "image/png";

/// One thing an answer is about: what to say, and the picture it names when
/// there is one.
///
/// Text, always: an answer a client cannot show as words is an answer nobody
/// can read back in a transcript. A picture *as well*, when the answer is one —
/// MCP carries an image as its own content block, so a client that can see
/// images sees the frame rather than a path to a file it has no way to open.
///
/// The two are not alternatives, and that is the point of the shape. A tool
/// that returned only an image would be unreadable to a client without vision
/// and unloggable everywhere; one that returned only text could never show a
/// frame at all.
pub struct Part {
    /// What to say. The whole answer for every tool that has nothing to show.
    pub text: String,
    /// A PNG, already base64-encoded, when there is a picture in this part.
    pub image: Option<String>,
}

impl Part {
    /// A picture, and the words that go with it.
    pub(crate) fn picture(text: String, png: &[u8]) -> Self {
        Self {
            text,
            image: Some(base64::encode(png)),
        }
    }
}

/// What a tool answers with: one part, or several in the order to read them.
///
/// Several, because a question is often asked about several things at once —
/// `still` handed six instants answers about all six — and the alternative is
/// a client paying for six round trips to ask one question. What the list
/// preserves is which words go with which picture: a block of prose about six
/// frames followed by six frames is unreadable, so each sentence sits directly
/// above the picture it names.
pub struct Reply {
    /// What the answer is made of, in the order a client renders it. Never
    /// empty — a reply that says nothing is not an answer.
    pub parts: Vec<Part>,
}

impl Reply {
    /// The content blocks MCP puts this on the wire as.
    ///
    /// Text first, always. A client renders blocks in order, and the sentence
    /// saying which frame this is belongs above the frame rather than under it.
    pub fn content(&self) -> Vec<Value> {
        let mut blocks = Vec::with_capacity(self.parts.len() * 2);
        for part in &self.parts {
            blocks.push(serde_json::json!({ "type": "text", "text": part.text }));
            if let Some(image) = &part.image {
                blocks.push(serde_json::json!({
                    "type": "image",
                    "data": image,
                    "mimeType": PNG
                }));
            }
        }
        blocks
    }
}

impl From<Vec<Part>> for Reply {
    fn from(parts: Vec<Part>) -> Self {
        Self { parts }
    }
}

impl From<String> for Reply {
    fn from(text: String) -> Self {
        Self {
            parts: vec![Part { text, image: None }],
        }
    }
}

impl From<&str> for Reply {
    fn from(text: &str) -> Self {
        Self::from(text.to_owned())
    }
}

/// What a tool needs to say about itself, and what it does.
pub trait Tool: Send + Sync {
    /// How a client names it. Stable — renaming one breaks every saved prompt
    /// that mentions it.
    fn name(&self) -> &'static str;

    /// What it does, in the words a client shows to whoever is deciding
    /// whether to call it.
    fn description(&self) -> &'static str;

    /// The JSON Schema of its arguments. Every property carries its own
    /// `description`, for the same reason the tool does.
    fn schema(&self) -> Value;

    /// Runs it. The `Ok` reply is what the client sees; the `Err` string is
    /// what it sees when the tool refused, which is just as much of an answer.
    ///
    /// A [`Reply`] is a `String` away — `Ok(text.into())` — so a tool with
    /// nothing to show says so in one word rather than in a struct literal.
    fn call(&self, arguments: &Value) -> Result<Reply, String>;
}

/// Every tool this server exposes.
pub fn registry() -> Vec<Box<dyn Tool>> {
    vec![
        // First, because it is the first call on a machine with no project on
        // it and a client reads this list in order.
        Box::new(create::New),
        Box::new(inspect::Read),
        Box::new(inspect::Describe),
        Box::new(inspect::Check),
        Box::new(inspect::Assets),
        Box::new(edit::Import),
        Box::new(edit::Probe),
        Box::new(script::Read),
        Box::new(script::Write),
        Box::new(edit::Write),
        Box::new(edit::Dissolve),
        Box::new(edit::Duck),
        Box::new(edit::ScalePacing),
        Box::new(synth::New),
        Box::new(synth::Read),
        Box::new(synth::Write),
        Box::new(synth::Set),
        Box::new(synth::Check),
        Box::new(synth::Bake),
        Box::new(synth::Survey),
        Box::new(level::Level),
        Box::new(edit::Render),
        Box::new(still::Still),
    ]
}

/// A tool by name.
pub fn find(name: &str) -> Option<Box<dyn Tool>> {
    registry().into_iter().find(|tool| tool.name() == name)
}

/// The project directory an argument object names.
///
/// Every tool takes one, and it is required rather than defaulted to the
/// working directory: a server started by a client has no meaningful working
/// directory, and guessing one is how you edit the wrong film.
pub(crate) fn project_dir(arguments: &Value) -> Result<std::path::PathBuf, String> {
    arguments
        .get("project")
        .and_then(Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "`project` is required: the path of the *.scor directory".to_owned())
}

/// The `project` property, spelled the same way in every tool's schema.
///
/// `project_new` writes its own, and only its own: the directory it names is
/// one to make rather than one to work on. The *name* of the argument is what
/// has to be shared, and it is.
pub(crate) fn project_property() -> Value {
    serde_json::json!({
        "type": "string",
        "description": "Path to the *.scor project directory to work on."
    })
}

/// A schema whose only argument is the project directory.
pub(crate) fn project_only_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": { "project": project_property() },
        "required": ["project"]
    })
}
