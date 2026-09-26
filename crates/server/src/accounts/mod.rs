//! Accounts: who may use the server, and how each request proves it (#533).
//!
//! - **A user** is an email and an argon2id password hash ([`password`]).
//!   There is no public sign-up: the operator creates accounts and resets
//!   passwords with `scorsese-server user …` ([`users`]), because the first
//!   users are friends and family and sign-up only makes sense once payment
//!   exists.
//! - **A browser** logs in once and holds a session cookie ([`sessions`]):
//!   an opaque random value, looked up in Postgres on every request.
//! - **Anything that is not a browser** — an MCP client, a script — sends a
//!   per-user bearer token ([`tokens`]), issued by the user and revocable one
//!   at a time.
//!
//! Sessions and tokens are both 32 random bytes of which only a SHA-256 is
//! stored ([`secret`]). Neither is signed, so the server holds **no signing
//! key** — there is no secret for `docs/credentials.md`'s resolver to find, and
//! nothing to rotate. The price is one indexed lookup per request, which at
//! this scale is nothing.

pub mod password;
pub mod secret;
pub mod sessions;
pub mod tokens;
pub mod users;

use std::path::PathBuf;

pub use users::Account;

/// Why an account operation did not happen.
#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    /// What was given is not an email address.
    #[error("{0:?} is not an email address")]
    InvalidEmail(String),

    /// Somebody already has an account under this email.
    #[error("there is already an account for {0}")]
    Taken(String),

    /// Nobody has an account under this email.
    #[error("there is no account for {0}")]
    NoSuchAccount(String),

    /// A password somebody chose is too short to be worth hashing.
    #[error("a password must be at least {} characters", password::MIN_LENGTH)]
    WeakPassword,

    /// The current password given to change it was wrong.
    #[error("the current password is not right")]
    WrongPassword,

    /// A token was asked for without a name to tell it apart by.
    #[error("a token needs a name, so it can be told apart when revoking it")]
    UnnamedToken,

    /// The operating system could not supply randomness, or argon2 failed.
    #[error("could not hash or generate a secret: {0}")]
    Crypto(String),

    /// The account's rows are gone but its files could not all be removed.
    ///
    /// Named with the directory, because the operator finishes the job by
    /// hand: the account no longer exists to delete again.
    #[error("the account is deleted, but its files at {} could not be removed: {source}", path.display())]
    Files {
        /// The user's directory under the storage root.
        path: PathBuf,
        /// What the filesystem said.
        #[source]
        source: std::io::Error,
    },

    /// The database refused or could not be reached.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}

/// `email`, trimmed and lower-cased, if it looks like an address at all.
///
/// Deliberately loose — one `@` with something either side and no spaces.
/// The operator types these for people they know; a stricter check would
/// only refuse real addresses.
pub fn normalize_email(email: &str) -> Result<String, AccountError> {
    let email = email.trim().to_lowercase();
    let well_formed = match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.is_empty()
                && !domain.contains('@')
                && !email.contains(char::is_whitespace)
        }
        None => false,
    };
    if well_formed {
        Ok(email)
    } else {
        Err(AccountError::InvalidEmail(email))
    }
}
