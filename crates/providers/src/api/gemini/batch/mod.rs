//! Gemini's **Batch API**: many drawings ordered at once, at half the price,
//! answered within a day (#894).
//!
//! Read 2026-10-08 off <https://ai.google.dev/gemini-api/docs/batch-mode>. A
//! batch covers only the `generateContent` method, so its requests are in
//! that method's shape ([`request`]) rather than the Interactions shape
//! [`super::request`] draws with now — the same model, the same sentence and
//! the same pictures, spelled the way the older endpoint spells them.
//!
//! Two calls, Veo's shape rather than a still's: one creates the job and
//! answers with its name at once ([`Gemini::batch`]); the other asks after the
//! name, and once the job has finished carries every picture back inline
//! ([`Gemini::batch_status`]). Requests go **inline**, which the page allows up
//! to 20 MB a job; a set larger than that is the caller's to split into
//! several jobs ([`INLINE_LIMIT`]). Nothing here knows about the sketch
//! lifecycle, the cache or a price.

pub mod request;
pub mod response;

use std::time::Duration;

use crate::api::http::{HttpError, encoded};

use super::{BASE, Gemini, Model};

/// The most one inline batch may weigh, in bytes of request body.
///
/// The page's limit is 20 MB; this leaves a megabyte for the envelope and for
/// the estimate being an estimate.
pub const INLINE_LIMIT: usize = 19 * 1024 * 1024;

/// How long creating a job may take: it is accepted, not drawn, so this bounds
/// the upload of up to [`INLINE_LIMIT`] of pictures and nothing more.
const CREATE_WAIT: Duration = Duration::from_secs(300);

/// The most a finished job's reply is read to, in bytes — every picture in it,
/// base64. Twenty 4K stills is about a gigabyte of base64 at worst; the jobs a
/// run creates are bounded by their requests' weight, not their pictures', so
/// this is generous rather than exact.
const MAX_RESULT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

impl Gemini {
    /// Creates a batch job of `body`'s requests on `model`, and answers with
    /// the job's name. **This is the call that spends the money**, half of it,
    /// whenever Google gets to the work.
    pub fn batch(
        &self,
        model: Model,
        body: &request::Create,
    ) -> Result<response::Operation, HttpError> {
        let url = format!("{BASE}/models/{}:batchGenerateContent", model.id());
        self.caller
            .post_large(&url, body, CREATE_WAIT, super::MAX_REPLY_BYTES)
    }

    /// Asks after the job `name` (`batches/…`), as the page's own polling loop
    /// does. Free.
    pub fn batch_status(&self, name: &str) -> Result<response::Operation, HttpError> {
        let url = format!("{BASE}/{}", path_of(name));
        let bytes = self.caller.download(&url, MAX_RESULT_BYTES)?;
        serde_json::from_slice(&bytes).map_err(|error| HttpError::Unreadable {
            url,
            message: error.to_string(),
        })
    }
}

/// A job's name as a URL path: `batches/` and the id, each segment encoded so
/// a name read back from a document cannot point the key anywhere else.
fn path_of(name: &str) -> String {
    let id = name.strip_prefix("batches/").unwrap_or(name);
    format!("batches/{}", encoded(id))
}

#[cfg(test)]
mod tests {
    use super::path_of;

    #[test]
    fn a_name_is_one_path_segment_under_batches() {
        assert_eq!(path_of("batches/abc123"), "batches/abc123");
        assert_eq!(path_of("abc123"), "batches/abc123");
        assert!(!path_of("batches/../models").contains("/../"));
    }
}
