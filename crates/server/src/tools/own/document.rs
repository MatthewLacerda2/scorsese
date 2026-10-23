//! `library_read`: a document from the caller's library, as text (#1510).
//!
//! A person hands over a script, a shot list or a briefing they wrote
//! elsewhere by uploading a `.txt` or `.md`; this is how the assistant reads
//! it. Reading a file is not an editing capability, so a tool only the web
//! has is no gap between the two: locally the person's own client reads any
//! file on their disk. The text is read on demand, never put in every turn,
//! so a long script is paid for once, when it is needed — and a page at a
//! time ([`PAGE_CHARS`]), so the largest document the library takes never
//! lands in a conversation whole and fills the model's context.

use scorsese_mcp::Reply;
use serde_json::{Value, json};

use super::super::{Caller, database};
use crate::library::{Kind, LibraryError};

/// The most characters one call answers with: about 12,000 tokens, a long
/// script whole, and a fraction of any model's context.
const PAGE_CHARS: usize = 48_000;

/// How a client names it.
pub(super) const NAME: &str = "library_read";

/// What it does.
pub(super) const SAYS: &str = "Read a text document from your library — a script, shot list, \
narration or briefing the person uploaded as a .txt or .md file — and answer with its whole \
text, or a page of it at a time when it is long: the reply then says where to call again \
`from`. Name it by the id `library` lists (kind text). Read one whenever the person mentions a \
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
            },
            "from": {
                "type": "integer",
                "minimum": 0,
                "description": "The character to start at, for the next page of a long \
                                document: the number the previous reply names. Default 0."
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
    let from = arguments.get("from").and_then(Value::as_u64).unwrap_or(0);
    let from = usize::try_from(from).unwrap_or(usize::MAX);
    Ok(page(id, &item.name, &text, from).into())
}

/// The page of `text` starting at character `from`, headed with what it is
/// and, unless it is the last, where the next one starts.
fn page(id: i64, name: &str, text: &str, from: usize) -> String {
    let total = text.chars().count();
    if from >= total && from > 0 {
        return format!("{id} “{name}” has {total} characters; there is nothing from {from}.");
    }
    let shown: String = text.chars().skip(from).take(PAGE_CHARS).collect();
    let to = from + shown.chars().count();
    if from == 0 && to == total {
        return format!("{id} “{name}”:\n\n{text}");
    }
    let rest = if to < total {
        format!("\n\n[continues: call library_read again with from: {to}]")
    } else {
        String::new()
    };
    format!("{id} “{name}”, characters {from}–{to} of {total}:\n\n{shown}{rest}")
}

#[cfg(test)]
mod tests {
    use super::{PAGE_CHARS, page};

    #[test]
    fn a_short_document_is_read_whole() {
        assert_eq!(page(3, "a.md", "héllo", 0), "3 “a.md”:\n\nhéllo");
    }

    #[test]
    fn a_long_one_is_read_a_page_at_a_time_on_character_boundaries() {
        let text = "é".repeat(PAGE_CHARS + 5);
        let first = page(3, "a.md", &text, 0);
        assert!(first.starts_with(&format!("3 “a.md”, characters 0–{PAGE_CHARS} of")));
        assert!(first.ends_with(&format!("with from: {PAGE_CHARS}]")));
        let last = page(3, "a.md", &text, PAGE_CHARS);
        assert!(last.ends_with(&format!("of {}:\n\n{}", PAGE_CHARS + 5, "é".repeat(5))));
        assert!(page(3, "a.md", &text, PAGE_CHARS + 5).contains("nothing from"));
    }
}
