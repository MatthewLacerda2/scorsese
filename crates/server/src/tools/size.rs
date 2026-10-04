//! What a tool's reply weighs in tokens (#707), estimated.
//!
//! A tool call spends no tokens itself; its reply is input to the model's next
//! call, which is where it is paid for. So "which tools are expensive" is a
//! question about replies, and `tool_calls.reply_tokens_estimate` is its
//! answer. An **estimate**, and the column says so: a vendor's own count is a
//! network round trip per call (on the turn's path, and rate-limited), and the
//! tokenizer of a user's own client over web MCP is not ours to know.
//!
//! The rules are the published rules of thumb: text at [`BYTES_PER_TOKEN`],
//! a picture at width × height / [`PIXELS_PER_TOKEN`] — Anthropic's figure,
//! read from the PNG's own header, since every picture a tool answers with is
//! one.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use scorsese_mcp::Reply;

/// Bytes of text per token: the usual figure for English and JSON.
const BYTES_PER_TOKEN: usize = 4;

/// Pixels per token of a picture, as Anthropic documents it.
const PIXELS_PER_TOKEN: u64 = 750;

/// What a picture whose size cannot be read is taken to weigh: Anthropic's
/// largest unscaled image, about 1.15 megapixels.
const UNREADABLE_PICTURE: u64 = 1_600;

/// `reply`'s estimated weight in tokens.
pub(super) fn reply(reply: &Reply) -> u64 {
    reply
        .parts
        .iter()
        .map(|part| text(&part.text) + part.image.as_deref().map_or(0, picture))
        .sum()
}

/// Words' estimated weight in tokens — a reply's, or a refusal's.
pub(super) fn text(text: &str) -> u64 {
    u64::try_from(text.len().div_ceil(BYTES_PER_TOKEN)).unwrap_or(u64::MAX)
}

/// A base64 PNG's estimated weight in tokens.
fn picture(base64: &str) -> u64 {
    match dimensions(base64) {
        Some((width, height)) => (width * height).div_ceil(PIXELS_PER_TOKEN),
        None => UNREADABLE_PICTURE,
    }
}

/// A base64 PNG's width and height, from its `IHDR` chunk: eight bytes of
/// signature, the chunk's length and name, then the two as big-endian `u32`s.
/// 32 base64 characters decode to exactly those 24 bytes.
fn dimensions(base64: &str) -> Option<(u64, u64)> {
    let header = STANDARD.decode(base64.get(..32)?).ok()?;
    if header.get(12..16)? != b"IHDR" {
        return None;
    }
    let read = |at: usize| -> Option<u64> {
        Some(u64::from(u32::from_be_bytes(
            header.get(at..at + 4)?.try_into().ok()?,
        )))
    };
    Some((read(16)?, read(20)?))
}

#[cfg(test)]
mod tests {
    use scorsese_mcp::Part;

    use super::*;

    /// A PNG's first 24 bytes, for a picture `width` by `height`, in base64.
    fn png(width: u32, height: u32) -> String {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes.extend([8, 6, 0, 0, 0]);
        STANDARD.encode(bytes)
    }

    #[test]
    fn text_is_four_bytes_a_token_rounded_up() {
        assert_eq!(text(""), 0);
        assert_eq!(text("abcd"), 1);
        assert_eq!(text("abcde"), 2);
    }

    #[test]
    fn a_picture_weighs_its_pixels_over_750() {
        assert_eq!(dimensions(&png(1280, 720)), Some((1280, 720)));
        assert_eq!(picture(&png(1280, 720)), 1229);
        assert_eq!(picture("not a picture at all, not even close"), 1_600);
        assert_eq!(picture("short"), 1_600);
    }

    #[test]
    fn a_reply_weighs_its_words_and_its_pictures() {
        let reply = Reply::from(vec![
            Part {
                text: "frame at 1s".into(),
                image: Some(png(30, 25)),
            },
            Part {
                text: "done".into(),
                image: None,
            },
        ]);
        assert_eq!(super::reply(&reply), 3 + 1 + 1);
    }
}
