//! Accounts: who may use the server, and how each request proves it (#533).
//!
//! - **A user** is a Google identity (#952): an email, and once they have
//!   signed in, Google's never-reused id for them (`google_sub`). There are no
//!   passwords. "Continue with Google" is sign-up and login at once — the
//!   first sign-in makes the account, or links one the operator made under
//!   the same verified email ([`users::sign_in`]) — and the operator can still
//!   create, link and delete accounts with `scorsese-server user …`.
//! - **A browser** signs in once and holds a session cookie ([`sessions`]):
//!   an opaque random value, looked up in Postgres on every request.
//! - **Anything that is not a browser** — an MCP client, a script — sends a
//!   per-user bearer token ([`tokens`]), issued by the user and revocable one
//!   at a time.
//!
//! Sessions and tokens are both 32 random bytes of which only a SHA-256 is
//! stored ([`secret`]). Neither is signed, so the server holds **no signing
//! key** — there is no secret for `docs/credentials.md`'s resolver to find, and
//! nothing to rotate. The price is one indexed lookup per request, which at
//! this scale is nothing. (The Google OAuth client's secret is Google's, not
//! ours: it proves the server to Google and signs nothing here.)

pub mod access;
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

    /// The account with this email is already linked to another Google
    /// account, so a sign-in with a different one may not take it over.
    #[error(
        "the account for {0} belongs to another Google account; \
         ask the operator to link this one"
    )]
    LinkedElsewhere(String),

    /// This Google account is already linked to another scorsese account —
    /// often one its first sign-in made, under a different email.
    #[error(
        "that Google account is already linked to account {0}; \
         if that account is empty, `user delete` it first"
    )]
    SubTaken(i64),

    /// A token was asked for without a name to tell it apart by.
    #[error("a token needs a name, so it can be told apart when revoking it")]
    UnnamedToken,

    /// The operating system could not supply randomness.
    #[error("could not generate a secret: {0}")]
    Crypto(String),

    /// The account's rows are gone but its files could not all be removed.
    ///
    /// Named with the directory, because the operator finishes the job by
    /// hand: the account no longer exists to delete again.
    #[error("the account is deleted, but its files at {} could not be removed: {source}", path.display())]
    Files {
        /// The user's directory under the storage or the cache root.
        path: PathBuf,
        /// What the filesystem said.
        #[source]
        source: std::io::Error,
    },

    /// The database refused or could not be reached.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}

impl AccountError {
    /// The error as the log may say it: an account is named by id alone, so
    /// the kinds carrying an email say their kind without it.
    pub fn logged(&self) -> String {
        match self {
            Self::InvalidEmail(_) => "not an email address".to_owned(),
            Self::Taken(_) => "the email already has an account".to_owned(),
            Self::NoSuchAccount(_) => "no account has the email".to_owned(),
            Self::LinkedElsewhere(_) => {
                "the email's account belongs to another Google account".to_owned()
            }
            other => other.to_string(),
        }
    }
}

/// `email`, trimmed and lower-cased, if it looks like an address at all.
///
/// Deliberately loose — one `@` with something either side and no spaces.
/// The operator types these for people they know, and Google has already
/// verified the rest; a stricter check would only refuse real addresses.
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

#[cfg(test)]
mod tests {
    use super::AccountError;

    #[test]
    fn the_log_never_names_an_email() {
        let email = "ana@example.com";
        for error in [
            AccountError::InvalidEmail(email.into()),
            AccountError::Taken(email.into()),
            AccountError::NoSuchAccount(email.into()),
            AccountError::LinkedElsewhere(email.into()),
        ] {
            assert!(error.to_string().contains(email), "{error}");
            assert!(!error.logged().contains(email), "{}", error.logged());
        }
        assert_eq!(
            AccountError::SubTaken(7).logged(),
            AccountError::SubTaken(7).to_string()
        );
    }
}
