//! A `multipart/form-data` body, written out by hand (RFC 7578).
//!
//! One vendor call takes one: ElevenLabs' Instant Voice Cloning
//! (`POST /v1/voices/add`) wants its recordings as files in a form, not as
//! JSON. `ureq` 3 has a multipart feature behind no flag scorsese turns on,
//! and turning one on for a single call buys a dependency `cargo deny` has to
//! keep clearing for what is fifty lines against a fixed specification — the
//! reason [`encoded`](super::encoded) is written here too. The body is built
//! whole in memory: a minute or two of somebody's voice is a few megabytes.
//!
//! **The boundary is derived from the content**, not drawn at random: a hash
//! of every part, so the same form is the same bytes (a test can hold them),
//! and checked against every part anyway — a boundary that occurred inside a
//! recording would cut it in two.

use std::time::Duration;

use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use super::{Caller, HttpError};

/// How long a form may take to send and be answered — a few minutes of
/// somebody's recordings going up on a slow line, which the one-minute
/// [`TIMEOUT`](super::TIMEOUT) would cut off.
const FORM_WHOLE: Duration = Duration::from_secs(5 * 60);

impl Caller {
    /// POSTs `form` as `multipart/form-data` and reads the reply as JSON — for
    /// a vendor that takes files: ElevenLabs' voice cloning.
    pub fn post_multipart<R: DeserializeOwned>(
        &self,
        url: &str,
        form: &Form,
    ) -> Result<R, HttpError> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(FORM_WHOLE))
            .http_status_as_error(false)
            .build()
            .into();
        let (content_type, body) = form.encode();
        let sent = self
            .signed(agent.post(url))
            .header("Content-Type", content_type)
            .send(&body[..]);
        self.json(url, sent)
    }
}

/// One part of a form.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    /// A text field.
    Text { name: String, value: String },
    /// A file: its field name, the file name sent with it, its media type and
    /// its bytes.
    File {
        name: String,
        file_name: String,
        media_type: String,
        bytes: Vec<u8>,
    },
}

/// A form being filled in, in the order its parts are sent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Form {
    parts: Vec<Part>,
}

impl Form {
    /// An empty form.
    pub fn new() -> Self {
        Self::default()
    }

    /// The same form, with the text field `name` set to `value`.
    pub fn text(mut self, name: &str, value: impl Into<String>) -> Self {
        self.parts.push(Part::Text {
            name: name.to_owned(),
            value: value.into(),
        });
        self
    }

    /// The same form, with a file under the field `name`. A field may hold
    /// several files: each is its own part, under the same name.
    pub fn file(mut self, name: &str, file_name: &str, media_type: &str, bytes: Vec<u8>) -> Self {
        self.parts.push(Part::File {
            name: name.to_owned(),
            file_name: file_name.to_owned(),
            media_type: media_type.to_owned(),
            bytes,
        });
        self
    }

    /// The `Content-Type` header and the body, ready to send.
    pub fn encode(&self) -> (String, Vec<u8>) {
        let boundary = self.boundary();
        let mut body = Vec::new();
        for part in &self.parts {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            match part {
                Part::Text { name, value } => {
                    body.extend_from_slice(
                        format!(
                            "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                            quoted(name)
                        )
                        .as_bytes(),
                    );
                    body.extend_from_slice(value.as_bytes());
                }
                Part::File {
                    name,
                    file_name,
                    media_type,
                    bytes,
                } => {
                    body.extend_from_slice(
                        format!(
                            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n\
                             Content-Type: {media_type}\r\n\r\n",
                            quoted(name),
                            quoted(file_name)
                        )
                        .as_bytes(),
                    );
                    body.extend_from_slice(bytes);
                }
            }
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        (format!("multipart/form-data; boundary={boundary}"), body)
    }

    /// A boundary no part contains: a hash of every part, rehashed in the
    /// astronomically unlikely case one does.
    fn boundary(&self) -> String {
        let mut hasher = Sha256::new();
        for part in &self.parts {
            match part {
                Part::Text { name, value } => {
                    hasher.update(name.as_bytes());
                    hasher.update(value.as_bytes());
                }
                Part::File {
                    name,
                    file_name,
                    bytes,
                    ..
                } => {
                    hasher.update(name.as_bytes());
                    hasher.update(file_name.as_bytes());
                    hasher.update(bytes);
                }
            }
        }
        let mut digest = hasher.finalize().to_vec();
        loop {
            let boundary = format!("scorsese-{}", scorsese_core::hex(&digest[..16]));
            if !self
                .parts
                .iter()
                .any(|part| part.contains(boundary.as_bytes()))
            {
                return boundary;
            }
            digest = Sha256::digest(&digest).to_vec();
        }
    }
}

impl Part {
    /// Whether `needle` occurs anywhere in what this part sends.
    fn contains(&self, needle: &[u8]) -> bool {
        let found = |haystack: &[u8]| haystack.windows(needle.len()).any(|at| at == needle);
        match self {
            Self::Text { name, value } => found(name.as_bytes()) || found(value.as_bytes()),
            Self::File {
                name,
                file_name,
                bytes,
                ..
            } => found(name.as_bytes()) || found(file_name.as_bytes()) || found(bytes),
        }
    }
}

/// `value` safe inside a quoted header parameter: a quote or a line break
/// would end the header early, so each is escaped as RFC 7578 §4.2 allows.
fn quoted(value: &str) -> String {
    value
        .replace('"', "%22")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_form_is_its_parts_between_one_boundary() {
        let form = Form::new()
            .text("name", "My voice")
            .file("files", "a.mp3", "audio/mpeg", b"ID3 one".to_vec())
            .file("files", "b.wav", "audio/wav", b"RIFF two".to_vec());
        let (content_type, body) = form.encode();
        let boundary = content_type
            .strip_prefix("multipart/form-data; boundary=")
            .expect("a multipart content type");
        let expected = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nMy voice\r\n\
             --{boundary}\r\nContent-Disposition: form-data; name=\"files\"; \
             filename=\"a.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\nID3 one\r\n\
             --{boundary}\r\nContent-Disposition: form-data; name=\"files\"; \
             filename=\"b.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFF two\r\n\
             --{boundary}--\r\n"
        );
        assert_eq!(String::from_utf8(body).unwrap(), expected);
    }

    #[test]
    fn the_same_form_is_the_same_bytes_and_another_is_not() {
        let form = || Form::new().text("name", "a");
        assert_eq!(form().encode(), form().encode());
        assert_ne!(form().encode().0, Form::new().text("name", "b").encode().0);
    }

    #[test]
    fn no_part_contains_the_boundary() {
        let first = Form::new().text("x", "y").boundary();
        // A part carrying the boundary the form would otherwise have chosen.
        let form =
            Form::new()
                .text("x", "y")
                .file("files", "f", "audio/mpeg", first.clone().into_bytes());
        let boundary = form.boundary();
        assert!(
            !form
                .parts
                .iter()
                .any(|part| part.contains(boundary.as_bytes()))
        );
    }

    #[test]
    fn a_needle_is_found_in_every_field_of_a_part_and_nowhere_else() {
        let text = |name: &str, value: &str| Part::Text {
            name: name.to_owned(),
            value: value.to_owned(),
        };
        let file = |name: &str, file_name: &str, bytes: &[u8]| Part::File {
            name: name.to_owned(),
            file_name: file_name.to_owned(),
            media_type: String::from("audio/mpeg"),
            bytes: bytes.to_vec(),
        };
        assert!(text("xNEEDLEx", "v").contains(b"NEEDLE"));
        assert!(text("n", "xNEEDLEx").contains(b"NEEDLE"));
        assert!(!text("n", "v").contains(b"NEEDLE"));
        assert!(file("xNEEDLE", "f", b"b").contains(b"NEEDLE"));
        assert!(file("n", "NEEDLE.mp3", b"b").contains(b"NEEDLE"));
        assert!(file("n", "f", b"..NEEDLE..").contains(b"NEEDLE"));
        assert!(!file("n", "f", b"b").contains(b"NEEDLE"));
    }

    #[test]
    fn a_quote_or_a_line_break_cannot_end_a_header() {
        assert_eq!(quoted("a\"b\r\nc"), "a%22b%0D%0Ac");
    }
}
