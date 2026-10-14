//! Users buying credits themselves (#548), through Stripe's hosted Checkout,
//! in reais.
//!
//! ## The flow
//!
//! 1. **A quote** ([`Payments::quote`]): the reais a person typed, what they
//!    buy in dollars of credit, and the rate that says so. Shown live as they
//!    type, so nobody pays without seeing the credits first.
//! 2. **A session** ([`begin`]): the same quote, with the rate **locked**
//!    into a `payments` row, and a Checkout Session at Stripe for exactly
//!    those reais, expiring in [`EXPIRES_AFTER_SECONDS`]. The browser goes to
//!    Stripe's page; the card, the bank and (once Stripe invites the account)
//!    Pix are Stripe's business from there.
//! 3. **Crediting** ([`settle::settle`]), exactly once, keyed on the session
//!    id. Two ways in, one function: Stripe's signed webhook, and the return
//!    page, whose call retrieves the session from Stripe in case the webhook
//!    is late. Whichever arrives second finds the row already `paid` under
//!    the same row lock, and a unique index on the ledger refuses a second
//!    top-up for one payment even if it did not.
//!
//! ## Money
//!
//! Reais are integer centavos, credits integer micro-dollars, and the rate is
//! stored as the text its source sent ([`scorsese_providers::fx`]): no float
//! holds any of it (`docs/web.md`, *Money*). Credits are the reais divided by
//! the rate, floored; Stripe's fee is absorbed, not passed on. A top-up only
//! ever adds to the balance.
//!
//! When neither rate source answers, nothing is started and the request is
//! refused with 503: a rate is never guessed.

mod begin;
pub mod settle;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use scorsese_providers::api::http::HttpError;
use scorsese_providers::credentials::{Environment, Secret};
use scorsese_providers::fx::{Cached, FxError, Live, Rates};
use scorsese_providers::payments::{Checkout, Stripe};

pub use begin::{Begun, Quote, begin};

use crate::config::{ConfigError, PUBLIC_ORIGIN, STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET};
use crate::credits::CreditError;

/// The least one purchase may be, in centavos: R$ 20, so Stripe's fixed
/// R$ 0,39 a card charge stays a small part of it.
pub const MINIMUM_CENTAVOS: i64 = 2_000;

/// The most one purchase may be, in centavos: R$ 2.000, which bounds what a
/// stolen card can spend here at once.
pub const MAXIMUM_CENTAVOS: i64 = 200_000;

/// How long a Checkout Session takes payment: thirty minutes, Stripe's
/// least, and one more so our clock running ahead of Stripe's cannot put the
/// expiry under its minimum.
pub const EXPIRES_AFTER_SECONDS: i64 = 31 * 60;

/// Stripe, the rate sources, and where browsers come back to.
#[derive(Clone)]
pub struct Payments {
    /// Where sessions are created and read: Stripe, or a test's stand-in.
    pub checkout: Arc<dyn Checkout>,
    /// Where the rate is read: the live sources, or a test's figure.
    pub rates: Arc<dyn Rates>,
    /// The webhook endpoint's signing secret, `whsec_…`.
    pub webhook_secret: Secret,
    /// The site's origin, with no trailing slash.
    pub origin: String,
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
            rates: Arc::new(Cached::new(Live)),
            webhook_secret: Secret::new(webhook.trim()),
            origin,
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
        if index > 0 && (whole.len() - index) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    format!("{grouped},{:02}", centavos % 100)
}

/// Seconds since the epoch.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(0))
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
