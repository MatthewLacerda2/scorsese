//! Issuing a token for a quote, and redeeming one.
//!
//! The store is a trait because there are two of them with nothing in common
//! but the record: a directory of files for a project on this machine, and the
//! hosted server's tables. The rules — how long, how many times, bound to what
//! — live here, once, so neither store can drift into a looser version of them.

use std::sync::atomic::{AtomicU64, Ordering};

use scorsese_core::hash_bytes;
use serde::{Deserialize, Serialize};

use super::{Quote, Spend};

/// How long a token stays good, in seconds: fifteen minutes.
///
/// Long enough for a person to read a quote, ask a question about it and say
/// yes. Short enough that a yes cannot be carried into a different
/// conversation hours later, when whoever gave it has forgotten what it was
/// for. The digest already refuses a changed brief; this refuses a stale
/// agreement, and quoting again costs nothing.
pub const LIFETIME_SECONDS: i64 = 15 * 60;

/// What a token was issued for — the whole record a store keeps.
///
/// Plain data, so a store can put it in a file or a row without knowing what
/// any of it means.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issued {
    /// What a client hands back to spend it.
    pub token: String,
    /// What kind of spending it is for.
    pub spend: Spend,
    /// The [`Quote::digest`] it is bound to.
    pub digest: String,
    /// What was quoted, in US cents — kept so a refusal can say what changed.
    pub cents: u64,
    /// When it was issued, in seconds since the Unix epoch.
    pub issued_at: i64,
    /// When it stops being good, in seconds since the Unix epoch.
    pub expires_at: i64,
}

/// Where issued tokens are kept until they are spent.
pub trait Quotes {
    /// What going wrong with the store looks like.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Keeps a freshly issued token.
    fn put(&self, issued: &Issued) -> Result<(), Self::Error>;

    /// Removes a token and answers what it was, or `None` if it is not there.
    ///
    /// **Removing is the point**: of two callers taking one token at once, one
    /// gets it and the other gets `None`, so a token is spent at most once.
    fn take(&self, token: &str) -> Result<Option<Issued>, Self::Error>;
}

/// Why a token was not honoured. Nothing was spent in any of these.
#[derive(Debug, thiserror::Error)]
pub enum Refused<E: std::error::Error + 'static> {
    /// Never issued, already used, or not a token at all.
    #[error(
        "`{token}` is not a live quote — it was never issued, or it has been used. \
         Nothing was sent; call again without confirm to be quoted afresh."
    )]
    Unknown {
        /// What was handed in.
        token: String,
    },
    /// Issued, and too long ago.
    #[error(
        "that quote expired {ago} seconds ago — a quote is good for {LIFETIME_SECONDS} \
         seconds. Nothing was sent; call again without confirm to be quoted afresh."
    )]
    Expired {
        /// How long ago it stopped being good.
        ago: i64,
    },
    /// Issued for the other kind of spending.
    #[error(
        "that quote was for {quoted}, not {asked}. Nothing was sent; call again without \
         confirm to be quoted for this."
    )]
    OtherSpend {
        /// What it was issued for.
        quoted: &'static str,
        /// What it was handed to.
        asked: &'static str,
    },
    /// What would be paid for is not what was quoted any more.
    #[error(
        "what this would spend has changed since it was quoted — a brief was edited, \
         added or re-priced (quoted {quoted}, now {now}). Nothing was sent; call again \
         without confirm, show the new quote, and confirm that one."
    )]
    Changed {
        /// What was quoted, as dollars.
        quoted: String,
        /// What it would cost now, as dollars.
        now: String,
    },
    /// The store itself failed.
    #[error("the quote store failed, so nothing was sent: {0}")]
    Store(#[source] E),
}

/// Issues a token for `quote`, good from `now` for [`LIFETIME_SECONDS`].
///
/// `now` is passed in rather than read, so expiry is testable without waiting
/// fifteen minutes.
pub fn issue<S: Quotes>(store: &S, quote: &Quote, now: i64) -> Result<Issued, S::Error> {
    let digest = quote.digest();
    let issued = Issued {
        token: mint(&digest, now),
        spend: quote.spend,
        digest,
        cents: quote.cents(),
        issued_at: now,
        expires_at: now.saturating_add(LIFETIME_SECONDS),
    };
    store.put(&issued)?;
    Ok(issued)
}

/// Spends `token` against `current`, the quote as it stands right now.
///
/// The token is consumed whatever the answer, and `current` must be computed
/// by the caller the way the call itself will decide — which is what makes
/// "bound to exactly what was quoted" true rather than hoped for.
pub fn redeem<S: Quotes>(
    store: &S,
    token: &str,
    current: &Quote,
    now: i64,
) -> Result<Issued, Refused<S::Error>> {
    let unknown = || Refused::Unknown {
        token: token.to_owned(),
    };
    let issued = store
        .take(token)
        .map_err(Refused::Store)?
        .ok_or_else(unknown)?;
    if now > issued.expires_at {
        return Err(Refused::Expired {
            ago: now - issued.expires_at,
        });
    }
    if issued.spend != current.spend {
        return Err(Refused::OtherSpend {
            quoted: issued.spend.as_str(),
            asked: current.spend.as_str(),
        });
    }
    if issued.digest != current.digest() {
        return Err(Refused::Changed {
            quoted: crate::prices::dollars(issued.cents),
            now: crate::prices::dollars(current.cents()),
        });
    }
    Ok(issued)
}

/// The prefix every token carries, so one is recognisable in a transcript.
const PREFIX: &str = "quote-";

/// How many hex characters follow the prefix.
const DIGITS: usize = 24;

/// Whether `token` has the shape of one this module mints.
///
/// A store is free to use the token as a file name or a key, so anything else
/// — a path, an empty string, a guess with a slash in it — is refused before it
/// reaches one.
pub(super) fn well_formed(token: &str) -> bool {
    token.strip_prefix(PREFIX).is_some_and(|digits| {
        digits.len() == DIGITS && digits.bytes().all(|b| b.is_ascii_hexdigit())
    })
}

/// A new token: unique rather than secret.
///
/// It needs no secrecy, because the store is what vouches for it — a token
/// that is not in the store is refused however it was made. It needs only to
/// never repeat, which the clock, the process and a counter see to.
fn mint(digest: &str, now: i64) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let seed = format!(
        "{digest}\n{now}\n{nanos}\n{}\n{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let hash = hash_bytes(seed.as_bytes());
    format!("{PREFIX}{}", &hash[..DIGITS])
}

#[cfg(test)]
mod tests {
    use super::{mint, well_formed};

    #[test]
    fn a_minted_token_is_well_formed_and_never_repeats() {
        let one = mint("d", 1);
        let two = mint("d", 1);
        assert!(well_formed(&one), "{one}");
        assert_ne!(one, two);
    }

    #[test]
    fn a_path_is_not_a_token() {
        assert!(!well_formed("../../project.json"));
        assert!(!well_formed("quote-"));
        assert!(!well_formed("quote-../../../../../../etc/pas"));
        assert!(!well_formed(""));
    }
}
