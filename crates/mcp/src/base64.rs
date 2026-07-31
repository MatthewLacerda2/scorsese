//! Base64, because MCP carries a picture as text.
//!
//! An image content block is `{"type":"image","data":"…","mimeType":"…"}`, and
//! `data` is the bytes of the file in standard base64 with padding — the
//! encoding RFC 4648 §4 defines. JSON has no other way to carry bytes, so this
//! is not a choice the server makes; it is what the protocol says.
//!
//! Written here rather than taken as a dependency, for the reason the crate doc
//! gives for hand-rolling the JSON-RPC above it: this is thirty lines with a
//! fixed specification and test vectors published alongside it, and every
//! dependency is one `cargo deny` has to keep clearing. Decoding is absent
//! because nothing here decodes — the server only ever sends pictures.

/// The alphabet, in the order the specification numbers it.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` as standard base64 with padding.
pub(crate) fn encode(bytes: &[u8]) -> String {
    // Four characters per three bytes, rounded up to a whole group: allocated
    // once, because a 1080p PNG is megabytes and growing a string into that a
    // character at a time is a copy per doubling.
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        // The group as one 24-bit number, short groups zero-filled — which is
        // what the padding then says was not really there.
        let bits = group.iter().enumerate().fold(0_u32, |bits, (index, byte)| {
            bits | u32::from(*byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            if index <= group.len() {
                let sextet = (bits >> (18 - 6 * index)) & 0b11_1111;
                encoded.push(char::from(ALPHABET[sextet as usize]));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The vectors from RFC 4648 §10, which exist so that an encoder can be
    /// checked against the specification rather than against itself.
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
            assert_eq!(encode(plain.as_bytes()), encoded, "encoding {plain:?}");
        }
    }

    /// The whole alphabet, including the two characters that differ between
    /// standard base64 and the URL-safe variant — a picture sent in the wrong
    /// one decodes to noise on the client.
    #[test]
    fn every_byte_reaches_the_standard_alphabet() {
        let all: Vec<u8> = (0..=255).collect();
        let encoded = encode(&all);
        assert!(
            encoded.contains('+') && encoded.contains('/'),
            "this must be standard base64, not the URL-safe alphabet: {encoded}"
        );
        assert!(
            encoded
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)),
            "something outside the alphabet got out: {encoded}"
        );
        assert_eq!(encoded.len(), 344, "256 bytes is 86 groups of four");
    }

    /// Padding is what makes the length recoverable, so it is worth asserting
    /// on its own: the encoding of any input is a multiple of four characters.
    #[test]
    fn the_encoding_is_always_a_whole_number_of_groups() {
        for length in 0..32 {
            let bytes = vec![0xAB; length];
            assert_eq!(encode(&bytes).len() % 4, 0, "{length} bytes");
        }
    }
}
