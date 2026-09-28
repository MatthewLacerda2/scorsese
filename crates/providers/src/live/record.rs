//! Keeping what a live check received, fit to become a fixture.
//!
//! The point of recording is that a fixture should be what a vendor really
//! sent, so the bodies are kept **as sent**: nothing is re-serialised, because
//! re-serialising reorders keys and reformats numbers, and a fixture that has
//! been through our own serde is a shape we imagined again. Scrubbing is
//! therefore textual — a value is found by parsing, then replaced in the
//! original text — and it removes exactly what earlier hand-captured fixtures
//! had removed by hand:
//!
//! - **the key itself**, wherever it appears;
//! - **account identifiers** — the values of the fields in [`ACCOUNT_FIELDS`];
//! - **signed URLs** — a URL whose query carries a signature or a token keeps
//!   its path and loses the query.
//!
//! Media bodies (an MP3, an MP4) are not written: they are not fixtures, and
//! a recording directory is for reading.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::api::tap::Exchange;

/// Fields whose values identify an account rather than describe a reply.
pub const ACCOUNT_FIELDS: &[&str] = &[
    "owner_id",
    "public_owner_id",
    "user_id",
    "account_id",
    "organization_id",
    "workspace_id",
];

/// Query parameters that make a URL a credential.
const SIGNED: &[&str] = &["signature", "token", "x-goog-", "key=", "expires"];

/// What replaces a scrubbed value.
pub const SCRUBBED: &str = "scrubbed";

/// `body` with the key, account ids and signed URL queries replaced.
///
/// JSON and server-sent events are scrubbed value by value; anything else
/// that is text has only the key replaced; bytes that are not text are
/// returned as they are, and never written.
pub fn scrub(body: &[u8], key: &str) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(body) else {
        return body.to_vec();
    };
    let mut text = if key.is_empty() {
        text.to_owned()
    } else {
        text.replace(key, SCRUBBED)
    };
    let documents: Vec<Value> = match serde_json::from_str::<Value>(&text) {
        Ok(value) => vec![value],
        Err(_) => text
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .filter_map(|data| serde_json::from_str(data.trim()).ok())
            .collect(),
    };
    let mut secrets = Vec::new();
    for document in &documents {
        collect(document, None, &mut secrets);
    }
    for (found, replacement) in secrets {
        text = text.replace(&found, &replacement);
    }
    text.into_bytes()
}

/// Every value in `value` to replace, as `(JSON text, replacement)`.
fn collect(value: &Value, field: Option<&str>, into: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => map.iter().for_each(|(k, v)| collect(v, Some(k), into)),
        Value::Array(items) => items.iter().for_each(|v| collect(v, field, into)),
        Value::String(text) => {
            let replacement = if field.is_some_and(|f| ACCOUNT_FIELDS.contains(&f)) {
                Some(SCRUBBED.to_owned())
            } else {
                unsigned(text)
            };
            if let Some(replacement) = replacement {
                into.push((quoted(text), quoted(&replacement)));
            }
        }
        _ => {}
    }
}

/// A signed URL without its query; `None` for anything else.
fn unsigned(text: &str) -> Option<String> {
    let (path, query) = text.split_once('?')?;
    let query = query.to_ascii_lowercase();
    (text.starts_with("http") && SIGNED.iter().any(|mark| query.contains(mark)))
        .then(|| format!("{path}?{SCRUBBED}"))
}

/// A string as JSON writes it, quotes and escapes included.
fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("a string serialises")
}

/// Writes each text body in `exchanges` into `dir`, named for the provider,
/// its order and status: `anthropic-1-messages-200.sse`. Returns what it
/// wrote, and says of each media body how big it was instead.
pub fn write(dir: &Path, provider: &str, exchanges: &[Exchange]) -> std::io::Result<Vec<String>> {
    std::fs::create_dir_all(dir)?;
    let mut said = Vec::new();
    for (index, exchange) in exchanges.iter().enumerate() {
        let name = format!(
            "{provider}-{}-{}-{}",
            index + 1,
            endpoint(&exchange.url),
            exchange.status
        );
        let Some(extension) = extension(&exchange.body) else {
            said.push(format!(
                "{name}: {} bytes of media, not written",
                exchange.body.len()
            ));
            continue;
        };
        let path: PathBuf = dir.join(format!("{name}.{extension}"));
        std::fs::write(&path, &exchange.body)?;
        said.push(path.display().to_string());
    }
    Ok(said)
}

/// The last segment of a URL's path, fit for a file name.
fn endpoint(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let last = path.rsplit('/').next().unwrap_or(path);
    last.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// `json`, `sse` or `txt` for a text body; `None` for media.
fn extension(body: &[u8]) -> Option<&'static str> {
    let text = std::str::from_utf8(body).ok()?;
    let start = text.trim_start();
    if start.starts_with("event:") || start.starts_with("data:") {
        Some("sse")
    } else if start.starts_with('{') || start.starts_with('[') {
        Some("json")
    } else if body.starts_with(b"ID3") {
        None
    } else {
        Some("txt")
    }
}
