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

mod edit;
mod inspect;

use serde_json::Value;

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

    /// Runs it. The `Ok` string is what the client sees; the `Err` string is
    /// what it sees when the tool refused, which is just as much of an answer.
    fn call(&self, arguments: &Value) -> Result<String, String>;
}

/// Every tool this server exposes.
pub fn registry() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(inspect::Read),
        Box::new(inspect::Describe),
        Box::new(inspect::Check),
        Box::new(inspect::Assets),
        Box::new(edit::Write),
        Box::new(edit::Duck),
        Box::new(edit::Render),
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
