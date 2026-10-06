//! A JSON document with its insignificant whitespace taken out.
//!
//! `project_read` hands the document to a model, which reads it and writes it
//! back; it is not for a person to read. Pretty-printed, about half of its
//! bytes are indentation, and every reply stays in the history every later
//! call of a conversation sends (#828).
//!
//! Done over the text rather than by parsing into a `Value` and serialising
//! again: `serde_json` here keeps object keys sorted, not in the order they
//! were written, so a round trip would reorder the document — and the reply
//! is meant to be the file on disk, only tighter. Dropping whitespace outside
//! strings changes nothing a parser sees.

/// `document` without the whitespace between its tokens, or `document` as it
/// is when it is not JSON at all.
///
/// The second case matters: a hand edit can leave the file unparseable, and
/// that is exactly when reading it matters most — the caller has to see what
/// is there to fix it, and stripping whitespace from text that is not JSON
/// would only make that harder.
pub(super) fn compact(document: &str) -> String {
    if serde_json::from_str::<serde::de::IgnoredAny>(document).is_err() {
        return document.to_owned();
    }
    let mut out = String::with_capacity(document.len());
    let mut in_string = false;
    let mut escaped = false;
    for c in document.chars() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            out.push(c);
        } else if !matches!(c, ' ' | '\t' | '\n' | '\r') {
            // The four characters JSON counts as whitespace, and no others.
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests;
