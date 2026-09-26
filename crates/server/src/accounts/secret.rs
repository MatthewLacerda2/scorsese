//! Session cookies and API tokens: random values of which only a digest is
//! kept.
//!
//! **SHA-256, not argon2.** A password is slow-hashed because people choose
//! guessable ones; these are 256 random bits, which no amount of hashing
//! speed makes guessable, and they are looked up on every request — a slow
//! hash there would be a denial of service against ourselves. The digest is
//! still worth storing instead of the value: a backup of the database, or a
//! row printed in a log, is then not a pile of live credentials.

use sha2::{Digest, Sha256};

use super::AccountError;

/// What every API token starts with.
///
/// So a token pasted into the wrong place is recognisable for what it is —
/// by a person, and by secret scanners that look for known prefixes.
pub const TOKEN_PREFIX: &str = "scor_";

/// A freshly minted secret: the value to hand out once, and its digest to
/// keep.
#[derive(Debug)]
pub struct Minted {
    /// The value, shown to its owner exactly once.
    pub plain: String,
    /// [`digest`] of `plain`, which is what the database holds.
    pub digest: Vec<u8>,
}

/// 32 random bytes as hex, after `prefix`.
pub fn mint(prefix: &str) -> Result<Minted, AccountError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| AccountError::Crypto(error.to_string()))?;
    let mut plain = String::with_capacity(prefix.len() + 64);
    plain.push_str(prefix);
    for byte in bytes {
        plain.push_str(&format!("{byte:02x}"));
    }
    let digest = digest(&plain);
    Ok(Minted { plain, digest })
}

/// The SHA-256 of a presented secret, to look it up by.
pub fn digest(plain: &str) -> Vec<u8> {
    Sha256::digest(plain.as_bytes()).to_vec()
}
