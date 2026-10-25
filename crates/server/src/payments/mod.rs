//! Users buying credits themselves (#548), through Stripe's hosted Checkout,
//! in reais.
//!
//! ## The flow
//!
//! 1. **A quote** ([`Payments::quote`]): the reais a person typed, what they
//!    buy in dollars of credit, the bonus a larger purchase adds ([`bonus`]),
//!    and the rate that says so. Shown live as they
//!    type, so nobody pays without seeing the credits first.
//! 2. **A session** ([`begin`]): the quote the person saw, with the rate **locked**
//!    into a `payments` row, and a Checkout Session at Stripe for exactly
//!    those reais, expiring in [`EXPIRES_AFTER_SECONDS`]. The browser goes to
//!    Stripe's page; the card, the bank and (once Stripe invites the account)
//!    Pix are Stripe's business from there.
//! 3. **Crediting** ([`settle::settle`]), exactly once, keyed on the session
//!    id. Two ways in, one function: Stripe's signed webhook, and the return
//!    page, whose call retrieves the session from Stripe in case the webhook
//!    is late. Whichever arrives second finds the row already `paid` under
//!    the same row lock, and a unique index on the ledger refuses a second
//!    top-up or bonus for one payment even if it did not.
//! 4. **Withdrawing** ([`withdraw`]), within 7 days, by the operator: what is
//!    left of the purchase, its bonus included, is taken back, and the reais
//!    to refund are printed for Stripe's Dashboard.
//!
//! ## Money
//!
//! Reais are integer centavos, credits integer micro-dollars, and the rate is
//! stored as the text its source sent ([`scorsese_providers::fx`]): no float
//! holds any of it (`docs/web.md`, *Money*). Credits are the reais divided by
//! the rate, floored; Stripe's fee is absorbed, not passed on. A top-up only
//! ever adds to the balance.
//!
//! ## The rate is kept, not fetched per purchase
//!
//! [`refresh`] reads it every two minutes (AwesomeAPI, then the PTAX); a
//! failed read keeps the last good one and is logged. A quote and a purchase
//! use the rate kept while it is at most ten minutes old, and are refused
//! with 503 past that: a rate is never guessed. A server just started reads
//! one before its first quote. A purchase names the rate the person was
//! shown, and is refused with 409 and the new quote when it has moved since,
//! so the rate stored on the payment is always the one they saw.

mod begin;
pub mod bonus;
mod refresh;
pub mod settle;
pub mod withdraw;

use std::sync::Arc;

use scorsese_providers::api::http::HttpError;
use scorsese_providers::credentials::{Environment, Secret};
use scorsese_providers::fx::{FxError, Latest, Live};
use scorsese_providers::payments::{Checkout, Stripe};

pub use begin::{Begun, Quote, begin};
pub use bonus::Bonus;
pub use refresh::refresh;

use crate::config::{ConfigError, PUBLIC_ORIGIN, STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET};
use crate::credits::CreditError;

/// The least one purchase may be, in centavos: R$ 20, so Stripe's fixed
/// R$ 0,39 a card charge stays a small part of it.
pub const MINIMUM_CENTAVOS: i64 = 2_000;

/// The most one purchase may be, in centavos: R$ 5.000 (the maintainer,
/// 2026-10-14), which bounds what a stolen card can spend here at once and
/// stays under Pix-on-Stripe's 3,000 USD a payment.
pub const MAXIMUM_CENTAVOS: i64 = 500_000;

/// How long a Checkout Session takes payment: thirty minutes, Stripe's
/// least, and one more so our clock running ahead of Stripe's cannot put the
/// expiry under its minimum.
pub const EXPIRES_AFTER_SECONDS: i64 = 31 * 60;

/// Stripe, the rate sources, and where browsers come back to.
#[derive(Clone)]
pub struct Payments {
    /// Where sessions are created and read: Stripe, or a test's stand-in.
    pub checkout: Arc<dyn Checkout>,
    /// The latest rate, refreshed in the background ([`refresh`]) from the
    /// live sources, or from a test's figure.
    pub rates: Arc<Latest>,
    /// The webhook endpoint's signing secret, `whsec_…`.
    pub webhook_secret: Secret,
    /// The site's origin, with no trailing slash.
    pub origin: String,
    /// The bonus credit larger purchases add (#1537).
    pub bonus: Bonus,
}

impl std::fmt::Debug for Payments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Payments")
            .field("origin", &self.origin)
            .finish_non_exhaustive()
    }
}

impl Payments {
    /// Stripe as the environment names it, if it names it whole: both keys
    /// and the public origin, or none, which is a server that sells nothing
    /// and answers a purchase with "not configured".
    pub fn from_environment(environment: &Environment) -> Result<Option<Self>, ConfigError> {
        let bonus = Bonus::from_environment(environment)?;
        let (Some(key), Some(webhook), Some(origin)) = (
            environment.get(STRIPE_SECRET_KEY),
            environment.get(STRIPE_WEBHOOK_SECRET),
            environment.get(PUBLIC_ORIGIN),
        ) else {
            return Ok(None);
        };
        let origin =
            crate::http::google::public_origin(origin).ok_or_else(|| ConfigError::Origin {
                value: origin.to_owned(),
            })?;
        Ok(Some(Self {
            checkout: Arc::new(Stripe::new(Secret::new(key.trim()))),
            rates: Arc::new(Latest::new(Arc::new(Live))),
            webhook_secret: Secret::new(webhook.trim()),
            origin,
            bonus,
        }))
    }
}

/// Why a payment was not started, read or credited.
#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    /// The amount is outside what one purchase may be.
    #[error(
        "a purchase is between R$ {} and R$ {}",
        reais(MINIMUM_CENTAVOS),
        reais(MAXIMUM_CENTAVOS)
    )]
    OutOfRange,
    /// No source gave a rate, so nothing was started.
    #[error(
        "the dollar's rate could not be read just now, so nothing was charged — try again in a minute"
    )]
    NoRate(#[source] FxError),
    /// The rate kept is older than it may be sold at, so nothing was started.
    #[error(
        "the dollar's rate has not been read for a while, so nothing was charged — try again in a minute"
    )]
    StaleRate,
    /// The rate moved between the quote the person saw and their paying;
    /// nothing was started, and this is the quote at the rate now.
    #[error("the dollar's rate just changed: check what your reais buy now and pay again")]
    RateMoved(Box<Quote>),
    /// Stripe refused or could not be reached.
    #[error("Stripe did not answer: {0}")]
    Stripe(#[from] HttpError),
    /// Stripe created a session with no page to send the browser to.
    #[error("Stripe created session {0} with no payment page")]
    NoPage(String),
    /// A call made off the async runtime never came back.
    #[error("a call to Stripe or a rate source was cut off")]
    Interrupted,
    /// There is no payment of the caller's by that session.
    #[error("no such payment")]
    NotFound,
    /// The ledger refused.
    #[error(transparent)]
    Credit(#[from] CreditError),
    /// The database refused.
    #[error("the database refused: {0}")]
    Database(#[from] sqlx::Error),
}

/// Centavos as reais the way a Brazilian writes them: `2.000,00`.
pub fn reais(centavos: i64) -> String {
    let whole = (centavos / 100).to_string();
    let mut grouped = String::new();
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    format!("{grouped},{:02}", centavos % 100)
}

/// Seconds since the epoch.
fn now() -> i64 {
    scorsese_core::Timestamp::unix_now().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reais_are_written_the_brazilian_way() {
        assert_eq!(reais(2_000), "20,00");
        assert_eq!(reais(5_005), "50,05");
        assert_eq!(reais(200_000), "2.000,00");
        assert_eq!(reais(123_456_789), "1.234.567,89");
    }
}
