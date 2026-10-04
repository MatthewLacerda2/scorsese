//! Server-sent events, as both chat vendors stream a reply.
//!
//! The stream is `event:` / `data:` lines, an event ending at a blank line.
//! Anthropic and Google both put a whole JSON object on each event's `data:`
//! lines, and both repeat whatever the `event:` line names inside it, so only
//! the data is read: one event, one object, parsed as `T`.

use std::io::BufRead;

use serde::de::DeserializeOwned;

/// Why a stream could not be read.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// The connection failed partway.
    #[error("the reply stopped arriving: {0}")]
    Io(#[from] std::io::Error),
    /// An event's data was not the JSON it should be.
    #[error("the reply carried something unreadable ({error}): {data}")]
    Unreadable {
        /// What the parser said.
        error: serde_json::Error,
        /// The data line.
        data: String,
    },
}

/// The events `reader` carries, each parsed as `T`, in order, until it ends.
pub fn events<T: DeserializeOwned, R: BufRead>(
    reader: R,
) -> impl Iterator<Item = Result<T, StreamError>> {
    let mut lines = reader.lines();
    std::iter::from_fn(move || {
        let mut data = String::new();
        loop {
            match lines.next() {
                None if data.is_empty() => return None,
                None => break,
                Some(Err(error)) => return Some(Err(error.into())),
                Some(Ok(line)) if line.trim_end().is_empty() && !data.is_empty() => break,
                Some(Ok(line)) => {
                    let line = line.strip_suffix('\r').unwrap_or(&line);
                    if let Some(more) = line.strip_prefix("data:") {
                        if !data.is_empty() {
                            data.push('\n');
                        }
                        data.push_str(more.strip_prefix(' ').unwrap_or(more));
                    }
                }
            }
        }
        Some(serde_json::from_str(&data).map_err(|error| StreamError::Unreadable { error, data }))
    })
}
