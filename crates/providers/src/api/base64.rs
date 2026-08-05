//! Base64, because JSON cannot carry bytes and one vendor sends audio anyway.
//!
//! ElevenLabs' Voice Design endpoint answers with three MP3 samples *inside*
//! the JSON reply — `audio_base_64`, standard base64 with padding, the encoding
//! RFC 4648 §4 defines. That is not a choice scorsese makes; it is what the
//! reply is, and a reply that cannot be decoded is three samples nobody can
//! hear.
//!
//! Written here rather than taken as a dependency, for the reason the MCP
//! server gives for its own encoder: this is thirty lines against a fixed
//! specification with published test vectors, and every dependency is one
//! `cargo deny` has to keep clearing. Encoding is absent because nothing here
//! encodes — this crate only ever *receives* bytes wrapped in text.
//!
//! **Permissive about whitespace, strict about everything else.** A server is
//! entitled to fold a long payload across lines, so newlines and spaces are
//! skipped; a character outside the alphabet is not folding, it is a reply that
//! is not what it claims to be, and decoding it to something plausible would
//! write a corrupt MP3 somebody would then try to play.

/// The alphabet, in the order the specification numbers it.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The bytes `text` encodes, or `None` if it is not standard base64.
///
/// `None` rather than a best effort: the caller is about to write the result to
/// disk as an audio file, and half a sample is worse than a refusal — it plays
/// as noise and looks like the provider's fault.
pub(crate) fn decode(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(text.len() / 4 * 3);
    // A 24-bit accumulator filled six bits at a time; every four sextets yield
    // three bytes, which is the whole of the encoding.
    let mut bits: u32 = 0;
    let mut filled = 0_u32;
    for byte in text.bytes() {
        if byte.is_ascii_whitespace() || byte == b'=' {
            continue;
        }
        let sextet = ALPHABET.iter().position(|it| *it == byte)?;
        bits = (bits << 6) | u32::try_from(sextet).ok()?;
        filled += 6;
        if filled == 24 {
            bytes.extend_from_slice(&bits.to_be_bytes()[1..]);
            bits = 0;
            filled = 0;
        }
    }
    // A trailing partial group: 12 bits carry one byte, 18 carry two, and 6 on
    // its own is not a group any encoder produces.
    match filled {
        0 => Some(bytes),
        12 => {
            bytes.push(u8::try_from((bits >> 4) & 0xFF).ok()?);
            Some(bytes)
        }
        18 => {
            bytes.push(u8::try_from((bits >> 10) & 0xFF).ok()?);
            bytes.push(u8::try_from((bits >> 2) & 0xFF).ok()?);
            Some(bytes)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The vectors from RFC 4648 §10, so the decoder is checked against the
    /// specification rather than against itself.
    #[test]
    fn the_specifications_own_test_vectors() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(
                decode(encoded).as_deref(),
                Some(plain.as_bytes()),
                "decoding {encoded:?}"
            );
        }
    }

    /// An MP3 is not text, so every byte value has to survive the round trip —
    /// including the two the URL-safe alphabet spells differently.
    #[test]
    fn every_byte_value_survives() {
        let all: Vec<u8> = (0..=255).collect();
        let encoded = concat!(
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4v",
            "MDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5f",
            "YGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6P",
            "kJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/",
            "wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v",
            "8PHy8/T19vf4+fr7/P3+/w=="
        );
        assert_eq!(decode(encoded).as_deref(), Some(all.as_slice()));
    }

    /// A vendor is entitled to fold a long payload; a vendor sending something
    /// else entirely is not something to paper over.
    #[test]
    fn whitespace_is_folding_and_anything_else_is_a_refusal() {
        assert_eq!(decode("Zm9v\nYmFy").as_deref(), Some(&b"foobar"[..]));
        assert_eq!(decode("Zm9v-mFy"), None, "a URL-safe payload is not this");
        assert_eq!(decode("<html>"), None, "an error page is not a sample");
    }
}
