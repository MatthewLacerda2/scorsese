//! `library_read`: a document from the caller's library, as text (#1510).
//!
//! A person hands over a script, a shot list or a briefing they wrote
//! elsewhere by uploading a `.txt` or `.md`; this is how the assistant reads
//! it. Reading a file is not an editing capability, so a tool only the web
//! has is no gap between the two: locally the person's own client reads any
//! file on their disk. The text is read on demand, never put in every turn,
//! so a long script is paid for once, when it is needed.

use scorsese_mcp::Reply;
use serde_json::{Value, json};

use super::super::{Caller, database};
use crate::library::{Kind, LibraryError};

/// How a client names it.
pub(super) const NAME: &str = "library_read";

/// What it does.
pub(super) const SAYS: &str = "Read a text document from your library — a script, shot list, \
narration or briefing the person uploaded as a .txt or .md file — and answer with its whole \
text. Name it by the id `library` lists (kind text). Read one whenever the person mentions a \
document they gave you, or when the library lists one that bears on the video: it is how they \
hand over what they wrote elsewhere. Only documents are read this way; media and pages are \
brought into a project with import.";

/// Its arguments.
pub(super) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "item": {
                "type": "integer",
                "description": "The document to read, by the id `library` lists."
            }
        },
        "required": ["item"]
    })
}

/// The caller's document `item`, as its text.
pub(super) async fn read(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let id = arguments
        .get("item")
        .and_then(Value::as_i64)
        .ok_or("`item` is required: the document's id, as `library` lists it")?;
    let library = &caller.toolbox.library;
    let item = library
        .get(caller.user, id)
        .await
        .map_err(|error| match error {
            LibraryError::NotFound => format!("there is no file {id} in your library"),
            other => database(other),
        })?;
    if item.kind != Kind::Text {
        return Err(format!(
            "{id} “{}” is a {}, not a document: library_read reads only text documents; \
             bring it into a project with import",
            item.name,
            item.kind.label()
        ));
    }
    let file = library
        .storage()
        .library_file(caller.user, &item.sha256, &item.extension);
    let text = tokio::fs::read_to_string(file).await.map_err(database)?;
    Ok(format!("{id} “{}”:\n\n{text}", item.name).into())
}
