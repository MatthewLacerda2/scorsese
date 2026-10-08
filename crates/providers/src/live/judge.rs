//! What a failed call means, vendor by vendor.
//!
//! Three of the five answers a check can give come out of a transport error,
//! and which one depends on the vendor: each says *your key is wrong* its own
//! way. Google answers a bad key with a `400` naming `API_KEY_INVALID`;
//! ElevenLabs uses `401` both for a bad key and for a good key missing a scope,
//! told apart only by the body ([`Refusal`]); Anthropic uses `401` and `403`.
//! Getting that wrong would send somebody to the wrong dashboard, so it is
//! decided here, once, and pinned by tests with the vendors' own bodies.

use crate::api::anthropic::stream::StreamError;
use crate::api::elevenlabs::refusal::Refusal;
use crate::api::http::HttpError;
use crate::claude::ClaudeError;

use super::Verdict;

/// A failed Gemini (Veo) call.
pub fn gemini(error: &HttpError) -> Verdict {
    sorted(error, |status, body| {
        matches!(status, 401 | 403) || body.contains("API_KEY_INVALID")
    })
}

/// A failed Anthropic call.
pub fn anthropic(error: &HttpError) -> Verdict {
    sorted(error, |status, _| matches!(status, 401 | 403))
}

/// A failed Claude call, which can also fail partway through its stream.
///
/// An event the parser would not take, a stream that stops before
/// `message_stop`, and tool arguments that do not join into JSON are all the
/// reply no longer being the shape [`crate::claude`] reads.
pub fn claude(error: &ClaudeError) -> Verdict {
    let shape = |field: String| Verdict::ShapeChanged { field };
    match error {
        ClaudeError::Http(error) => anthropic(error),
        ClaudeError::Stream(StreamError::Io(error)) => Verdict::Unreachable {
            said: error.to_string(),
        },
        ClaudeError::Stream(StreamError::Unreadable { error, data }) => {
            let data: String = data.chars().take(300).collect();
            shape(format!(
                "an event the client could not read ({error}): {data}"
            ))
        }
        ClaudeError::Api { kind, message } => Verdict::Refused {
            said: format!("the API failed partway ({kind}): {message}"),
        },
        ClaudeError::Cut => shape(String::from("the stream ended before message_stop")),
        ClaudeError::Arguments(error) => shape(format!(
            "tool_use: the input_json_delta fragments did not join into JSON ({error})"
        )),
    }
}

/// A failed ElevenLabs call — sorted by [`Refusal::of`], the one place that
/// reads this vendor's refusals, so a missing scope is reported as the key
/// being fine and the permission absent, in the vendor's words.
pub fn elevenlabs(error: &HttpError) -> Verdict {
    let HttpError::Refused { status, body, .. } = error else {
        return sorted(error, |_, _| false);
    };
    let refusal = Refusal::of(*status, "", body);
    let said = refusal.to_string();
    match refusal {
        Refusal::BadKey { .. } | Refusal::MissingPermission { .. } => Verdict::AuthFailed { said },
        _ => Verdict::Refused { said },
    }
}

/// The verdict for `error`, with `auth` deciding which refusals are about the
/// key.
///
/// An unreadable reply is a **shape change**: the vendor answered `2xx` with
/// something our serde types would not take, and serde's message names the
/// field — *missing field `name`* — which is the whole of what the report
/// needs to say.
fn sorted(error: &HttpError, auth: impl Fn(u16, &str) -> bool) -> Verdict {
    match error {
        HttpError::Unreachable { message, .. } => Verdict::Unreachable {
            said: message.clone(),
        },
        HttpError::Unreadable { message, .. } => Verdict::ShapeChanged {
            field: message.clone(),
        },
        HttpError::Refused { status, body, .. } if auth(*status, body) => Verdict::AuthFailed {
            said: format!("{status}: {body}"),
        },
        HttpError::Refused { status, body, .. } => Verdict::Refused {
            said: format!("{status}: {body}"),
        },
    }
}

/// Whether `bytes` look like an MP3: an ID3 tag, or an MPEG audio frame sync.
pub fn is_mp3(bytes: &[u8]) -> bool {
    bytes.starts_with(b"ID3") || matches!(bytes, [0xFF, second, ..] if second & 0xE0 == 0xE0)
}

/// Whether `bytes` look like an MP4: an ISO media `ftyp` box first.
pub fn is_mp4(bytes: &[u8]) -> bool {
    bytes.get(4..8) == Some(b"ftyp".as_slice())
}

/// The width and height of a JPEG, read off its frame header; `None` for
/// anything that is not one.
///
/// A JPEG is a run of segments, each a `0xFF` marker byte and a big-endian
/// length that counts itself. The size sits in the start-of-frame segment
/// (`SOF0`–`SOF15`, less the three markers in that range that are not frames:
/// `DHT`, `JPG` and `DAC`), height then width as `u16`s after one precision
/// byte. So walking the segment headers is the whole of what telling a
/// picture's size takes, and decoding the pixels would prove nothing the
/// check needs.
pub fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut rest = bytes.strip_prefix(&[0xFF, 0xD8])?;
    loop {
        let [0xFF, marker, high, low, ..] = *rest else {
            return None;
        };
        let length = usize::from(u16::from_be_bytes([high, low]));
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let frame = rest.get(4..9)?;
            let number = |at: usize| u32::from(u16::from_be_bytes([frame[at], frame[at + 1]]));
            return Some((number(3), number(1)));
        }
        rest = rest.get(2 + length..)?;
    }
}

/// What a body that should have been media looks like instead, for a report.
pub(super) fn looks_like(bytes: &[u8]) -> String {
    match bytes.first() {
        None => String::from("an empty body"),
        Some(b'{' | b'[') => String::from("JSON"),
        Some(_) => format!(
            "{} bytes starting {:02X?}",
            bytes.len(),
            &bytes[..bytes.len().min(8)]
        ),
    }
}
