//! An [`HttpError`] in the words that reach an agent, and whether it is worth
//! asking again — answered once for every vendor (#1290).
//!
//! Every client met the same two questions on its own: Veo worded a 429 while
//! Gemini, on the very same Google quota, handed the agent Google's page of
//! JSON; Pixabay and LottieFiles each mapped the status again; and the list of
//! statuses worth retrying lived in the Claude client alone. A vendor module
//! still decides what a refusal means **on its own endpoint** (a lost Veo
//! ticket, a Pixabay key refused); what every vendor's refusal means alike is
//! here.

use crate::credentials::Secret;

use super::HttpError;

/// What a 429 says, whoever sent it. Its body is a vendor's page of JSON that
/// says, once read, exactly this — and it is the refusal a run of several
/// shots or stills meets most (#1057).
pub const AT_ITS_LIMIT: &str =
    "at its limit — too many requests on this key, or its quota spent (429)";

/// Stands where a key was scrubbed out of a message.
const SCRUBBED: &str = "<key>";

impl HttpError {
    /// The status the server answered, if it answered at all.
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Refused { status, .. } => Some(*status),
            Self::Unreachable { .. } | Self::Unreadable { .. } => None,
        }
    }

    /// Whether the vendor said too many requests, or a spent quota.
    pub fn rate_limited(&self) -> bool {
        self.status() == Some(429)
    }

    /// Whether the same request is worth sending again: the connection
    /// failed, or the vendor was busy or failed on its side — never a request
    /// it refused for what it said, and never a reply that arrived unreadable,
    /// which would arrive unreadable again.
    pub fn retryable(&self) -> bool {
        match self {
            Self::Unreachable { .. } => true,
            Self::Refused { status, .. } => {
                matches!(status, 408 | 409 | 429 | 500..=504 | 529)
            }
            Self::Unreadable { .. } => false,
        }
    }

    /// The error as a sentence for the agent: a 429 in [`AT_ITS_LIMIT`]'s
    /// words, anything else as the transport and the vendor put it — the body
    /// is usually the sentence that says what to change. `key`, where given,
    /// never appears in it.
    pub fn explained(&self, key: Option<&Secret>) -> String {
        if self.rate_limited() {
            return String::from(AT_ITS_LIMIT);
        }
        scrubbed(&self.to_string(), key)
    }
}

/// `text` with `key` taken out of it.
///
/// A key should never be in a message — a query-string key is added to the
/// request, never to the URL an error names — and is scrubbed anyway, because
/// a key in a log is not undone. An empty key scrubs nothing: replacing `""`
/// would put the marker between every character.
pub fn scrubbed(text: &str, key: Option<&Secret>) -> String {
    match key.map(Secret::expose) {
        Some(key) if !key.is_empty() => text.replace(key, SCRUBBED),
        _ => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What Google answers a key past its quota, abridged from a live 429.
    const GOOGLE_429: &str = r#"{"error":{"code":429,"message":"Resource has been exhausted (e.g. check quota).","status":"RESOURCE_EXHAUSTED"}}"#;

    fn refused(status: u16, body: &str) -> HttpError {
        HttpError::Refused {
            url: String::from("https://vendor.example/v1/call"),
            status,
            body: String::from(body),
        }
    }

    #[test]
    fn a_429_is_the_sentence_never_the_page() {
        let said = refused(429, GOOGLE_429).explained(None);
        assert_eq!(said, AT_ITS_LIMIT);
        assert!(!said.contains("RESOURCE_EXHAUSTED"), "{said}");
    }

    #[test]
    fn any_other_refusal_keeps_the_vendors_words() {
        let said = refused(400, "1080p is not supported for 4 seconds").explained(None);
        assert!(said.contains("answered 400"), "{said}");
        assert!(said.contains("1080p is not supported"), "{said}");
    }

    #[test]
    fn a_key_never_survives_into_the_words() {
        let key = Secret::new("sekrit");
        let unreachable = HttpError::Unreachable {
            url: String::from("https://pixabay.com/api/?key=sekrit"),
            message: String::from("dns error"),
        };
        let said = unreachable.explained(Some(&key));
        assert!(!said.contains("sekrit"), "{said}");
        assert!(said.contains("key=<key>"), "{said}");
        let refusal = refused(500, "bad key sekrit").explained(Some(&key));
        assert!(!refusal.contains("sekrit"), "{refusal}");
    }

    #[test]
    fn an_empty_key_scrubs_nothing() {
        assert_eq!(scrubbed("as said", Some(&Secret::new(""))), "as said");
        assert_eq!(scrubbed("as said", None), "as said");
    }

    #[test]
    fn busy_and_broken_are_retried_and_refusals_are_not() {
        for status in [408, 409, 429, 500, 502, 503, 504, 529] {
            assert!(refused(status, "").retryable(), "{status}");
        }
        for status in [400, 401, 403, 404, 422, 505] {
            assert!(!refused(status, "").retryable(), "{status}");
        }
        let unreachable = HttpError::Unreachable {
            url: String::new(),
            message: String::new(),
        };
        let unreadable = HttpError::Unreadable {
            url: String::new(),
            message: String::new(),
        };
        assert!(unreachable.retryable() && !unreadable.retryable());
        assert_eq!(unreachable.status(), None);
        assert!(refused(429, "").rate_limited() && !refused(503, "").rate_limited());
    }
}
