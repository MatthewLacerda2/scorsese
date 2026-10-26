//! Content hashing for the media pool.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

/// Streams a file through SHA-256 and returns lowercase hex.
///
/// Streamed rather than read whole: a source video can be gigabytes, and
/// import must not need it all in memory at once. Public because a file is
/// named by this hash wherever it is kept — a project's pool here, a user's
/// library in `scorsese-server` — and two hashers would be two answers.
pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

/// Hashes bytes already in memory, returning the same lowercase hex a hash of
/// a whole file does.
///
/// For content that is small by construction and is being hashed to *name*
/// something rather than to verify it — a synthesis recipe, whose hash is the
/// file name of its bake.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex(&hasher.finalize())
}

/// `bytes` as lowercase hex, two digits a byte (#1522).
///
/// The one spelling of a digest, a key or a random token wherever one is
/// written as text.
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::{hash_bytes, hex};

    #[test]
    fn hex_is_lowercase_and_two_digits_a_byte() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
        assert_eq!(hex(&[]), "");
    }

    #[test]
    fn a_hash_is_the_hex_of_its_digest() {
        // SHA-256 of nothing.
        assert_eq!(
            hash_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
