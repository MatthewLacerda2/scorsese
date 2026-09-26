//! Passwords: argon2id to store them, and generating one for the operator.
//!
//! **argon2id with the crate's defaults** — 19 MiB, two passes, one lane,
//! which is OWASP's recommended minimum. The parameters are written into each
//! stored hash (the PHC string), so raising them later re-hashes nobody and
//! breaks nobody: old hashes verify with their own parameters.
//!
//! Hashing is deliberately slow, tens of milliseconds, so callers on the async
//! runtime run it through [`hash_blocking`] and [`verify_blocking`] rather
//! than stalling a worker thread every request shares.

use std::sync::OnceLock;

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};

use super::AccountError;

/// The shortest password a user may choose for themselves.
///
/// Eight, per NIST SP 800-63B: length is what matters, and a longer minimum
/// mostly produces passwords written on a sticky note.
pub const MIN_LENGTH: usize = 8;

/// The characters a generated password is made of: no `0`/`o`, `1`/`l`/`i`,
/// because the operator reads it out or pastes it into a message.
const ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

/// How long a generated password is: 20 characters from [`ALPHABET`] is
/// about 99 bits, which no amount of guessing at a login form reaches.
const GENERATED_LENGTH: usize = 20;

/// The PHC string for `password`, with a fresh random salt.
pub fn hash(password: &str) -> Result<String, AccountError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|error| AccountError::Crypto(error.to_string()))
}

/// Whether `password` is the one `stored` was made from.
///
/// A stored hash that does not parse is a no, not an error: the only way to
/// get one is to write the column by hand, and refusing the login is the
/// answer that fails safe.
pub fn verify(password: &str, stored: &str) -> bool {
    PasswordHash::new(stored).is_ok_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

/// [`hash`], off the async runtime.
pub async fn hash_blocking(password: String) -> Result<String, AccountError> {
    tokio::task::spawn_blocking(move || hash(&password))
        .await
        .map_err(|error| AccountError::Crypto(error.to_string()))?
}

/// [`verify`], off the async runtime.
///
/// With `stored` absent — nobody has that email — it still hashes once
/// against a throwaway hash, so a wrong email takes as long to refuse as a
/// wrong password and the response time does not say which accounts exist.
pub async fn verify_blocking(password: String, stored: Option<String>) -> bool {
    tokio::task::spawn_blocking(move || match stored {
        Some(stored) => verify(&password, &stored),
        None => {
            verify(&password, decoy());
            false
        }
    })
    .await
    .unwrap_or(false)
}

/// A valid hash of nothing anybody will type, made once per process.
fn decoy() -> &'static str {
    static DECOY: OnceLock<String> = OnceLock::new();
    DECOY.get_or_init(|| hash("decoy: no account has this password").unwrap_or_default())
}

/// A new random password, for the operator to hand to somebody.
pub fn generate() -> Result<String, AccountError> {
    // Rejection sampling keeps every character equally likely: 248 is the
    // largest multiple of the alphabet's 31 that fits in a byte.
    let limit = (256 / ALPHABET.len() * ALPHABET.len()) as u8;
    let mut password = String::with_capacity(GENERATED_LENGTH);
    let mut bytes = [0u8; 64];
    while password.len() < GENERATED_LENGTH {
        getrandom::fill(&mut bytes).map_err(|error| AccountError::Crypto(error.to_string()))?;
        for &byte in bytes.iter().filter(|&&byte| byte < limit) {
            if password.len() == GENERATED_LENGTH {
                break;
            }
            password.push(char::from(ALPHABET[usize::from(byte) % ALPHABET.len()]));
        }
    }
    Ok(password)
}

/// `password` if a user may choose it, or why not.
pub fn acceptable(password: &str) -> Result<&str, AccountError> {
    if password.chars().count() >= MIN_LENGTH {
        Ok(password)
    } else {
        Err(AccountError::WeakPassword)
    }
}
