//! Taking a user's money for credits (#548): Stripe's hosted Checkout, behind
//! a trait so no test reaches Stripe.
//!
//! The server creates a session for an amount in reais and sends the browser
//! to Stripe; Stripe takes the payment and tells the server how it went, by a
//! signed webhook ([`webhook`]) and by the session itself when the browser
//! comes back and the server retrieves it. Which of those arrives first is a
//! race the server settles; this module only speaks to Stripe.
//!
//! What a credit is worth, the ledger and who paid are the server's. This
//! crate holds the HTTP client and the vendor's wire (`api::stripe`), so the
//! call is made here, the way [`crate::sign_in`] trades a code for Google.

pub mod webhook;

use crate::api::http::HttpError;
use crate::api::stripe::{self, Create};
use crate::credentials::Secret;

pub use crate::api::stripe::{SESSION_ID, Session};

/// What a Checkout Session is for: one charge in reais.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// What is charged, in centavos.
    pub centavos: i64,
    /// The line's name on Stripe's page — what the reais buy.
    pub name: String,
    /// Where the browser goes once paid; may carry [`SESSION_ID`].
    pub success_url: String,
    /// Where the browser goes if the person backs out.
    pub cancel_url: String,
    /// Seconds since the epoch at which the session stops taking payment.
    pub expires_at: i64,
    /// Ours, shown beside the session in Stripe's Dashboard.
    pub reference: String,
}

/// Where sessions are created and read: Stripe, or a test's stand-in.
///
/// Blocking: each is one HTTPS call, made off the async runtime by whoever
/// holds this.
pub trait Checkout: Send + Sync {
    /// Open a session for `order`.
    fn create(&self, order: &Order) -> Result<Session, HttpError>;

    /// The session `id` as it is now.
    fn retrieve(&self, id: &str) -> Result<Session, HttpError>;
}

/// Stripe itself, with the account's secret key.
#[derive(Debug, Clone)]
pub struct Stripe {
    key: Secret,
}

impl Stripe {
    /// Stripe, called with `key` (`sk_test_…` or `sk_live_…`).
    pub fn new(key: Secret) -> Self {
        Self { key }
    }
}

impl Checkout for Stripe {
    fn create(&self, order: &Order) -> Result<Session, HttpError> {
        stripe::create(
            &self.key,
            &Create {
                currency: "brl",
                unit_amount: order.centavos,
                name: &order.name,
                success_url: &order.success_url,
                cancel_url: &order.cancel_url,
                expires_at: order.expires_at,
                client_reference_id: &order.reference,
            },
        )
    }

    fn retrieve(&self, id: &str) -> Result<Session, HttpError> {
        stripe::retrieve(&self.key, id)
    }
}

/// Whether `session` says the money arrived: `paid`, in reais, exactly
/// `centavos`. Anything else credits nothing — an amount that disagrees with
/// what was asked for is somebody's mistake to look at, not a top-up.
pub fn paid(session: &Session, centavos: i64) -> bool {
    session.payment_status == "paid"
        && session.amount_total == Some(centavos)
        && session.currency.as_deref() == Some("brl")
}

/// Whether `session` can no longer be paid.
pub fn expired(session: &Session) -> bool {
    session.status.as_deref() == Some("expired")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(payment_status: &str, amount: i64, currency: &str) -> Session {
        Session {
            id: "cs_test_1".into(),
            url: None,
            status: Some("complete".into()),
            payment_status: payment_status.into(),
            amount_total: Some(amount),
            currency: Some(currency.into()),
        }
    }

    #[test]
    fn only_the_amount_asked_for_in_reais_and_paid_is_paid() {
        assert!(paid(&session("paid", 5_000, "brl"), 5_000));
        assert!(!paid(&session("unpaid", 5_000, "brl"), 5_000));
        assert!(!paid(&session("paid", 4_999, "brl"), 5_000));
        assert!(!paid(&session("paid", 5_000, "usd"), 5_000));
    }

    #[test]
    fn only_an_expired_status_is_expired() {
        let mut open = session("unpaid", 5_000, "brl");
        open.status = Some("open".into());
        assert!(!expired(&open));
        open.status = Some("expired".into());
        assert!(expired(&open));
    }
}
