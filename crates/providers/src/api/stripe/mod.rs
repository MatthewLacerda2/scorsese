//! Stripe's Checkout Sessions: the hosted page a user pays for credits on
//! (#548), and the webhook events that say how it went.
//!
//! Two calls and one event. A session is **created** with the amount and
//! where to come back to, and the browser is sent to its `url`; a session is
//! **retrieved** by id when the browser comes back, in case the webhook has
//! not arrived yet; and an **event** carries a session to the webhook when it
//! completes or expires.
//!
//! What is particular to this vendor, checked against
//! <https://docs.stripe.com/api/checkout/sessions/create>,
//! <https://docs.stripe.com/api/checkout/sessions/object> and
//! <https://docs.stripe.com/webhooks> on 2026-10-14:
//!
//! - **Requests are forms**, `application/x-www-form-urlencoded`, with nested
//!   parameters written `line_items[0][price_data][currency]`; replies are
//!   JSON. The secret key travels as `Authorization: Bearer sk_…`.
//! - **`expires_at` may be 30 minutes to 24 hours after creation.** The
//!   bound is checked against Stripe's clock, not ours.
//! - **`payment_method_types` is left out on purpose.** Without it the
//!   session offers what the Dashboard has switched on, which is how Pix is
//!   turned on when Stripe's invitation arrives — no code change.
//! - **A session's `status` is `open`, `complete` or `expired`, and its
//!   `payment_status` is `paid`, `unpaid` or `no_payment_required`.** Only
//!   `paid` means the money arrived: a session can be `complete` and `unpaid`
//!   while a delayed method settles, and `async_payment_succeeded` follows.
//! - `success_url` may carry `{CHECKOUT_SESSION_ID}`, which Stripe replaces
//!   with the session's id when it sends the browser back.

use serde::Deserialize;

use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// Every Stripe endpoint hangs off this.
const BASE: &str = "https://api.stripe.com/v1";

/// The placeholder Stripe replaces with the session's id in `success_url`.
pub const SESSION_ID: &str = "{CHECKOUT_SESSION_ID}";

/// A one-item hosted Checkout Session, in Stripe's parameter names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Create<'a> {
    /// The ISO currency, lowercase: `brl`.
    pub currency: &'a str,
    /// What is charged, in the currency's smallest unit (centavos).
    pub unit_amount: i64,
    /// The line's name on the hosted page.
    pub name: &'a str,
    /// Where the browser goes once paid; may carry [`SESSION_ID`].
    pub success_url: &'a str,
    /// Where the browser goes if the person backs out.
    pub cancel_url: &'a str,
    /// Seconds since the epoch at which the session stops taking payment.
    pub expires_at: i64,
    /// Ours, shown in the Dashboard beside the session: the account's id.
    pub client_reference_id: &'a str,
}

impl Create<'_> {
    /// The form, in the order Stripe's reference lists the fields.
    pub fn form(&self) -> Vec<(&'static str, String)> {
        vec![
            ("mode", "payment".to_owned()),
            ("cancel_url", self.cancel_url.to_owned()),
            ("client_reference_id", self.client_reference_id.to_owned()),
            ("expires_at", self.expires_at.to_string()),
            (
                "line_items[0][price_data][currency]",
                self.currency.to_owned(),
            ),
            (
                "line_items[0][price_data][product_data][name]",
                self.name.to_owned(),
            ),
            (
                "line_items[0][price_data][unit_amount]",
                self.unit_amount.to_string(),
            ),
            ("line_items[0][quantity]", "1".to_owned()),
            ("success_url", self.success_url.to_owned()),
        ]
    }
}

/// A Checkout Session: only the part read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Session {
    /// `cs_test_…` or `cs_live_…`.
    pub id: String,
    /// The hosted page, while the session is open.
    #[serde(default)]
    pub url: Option<String>,
    /// `open`, `complete` or `expired`; nullable in Stripe's reference.
    #[serde(default)]
    pub status: Option<String>,
    /// `paid`, `unpaid` or `no_payment_required`.
    pub payment_status: String,
    /// What was charged in total, in the smallest unit.
    #[serde(default)]
    pub amount_total: Option<i64>,
    /// The ISO currency, lowercase.
    #[serde(default)]
    pub currency: Option<String>,
}

/// A webhook event: only the part read.
#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    /// `evt_…`.
    pub id: String,
    /// What happened: `checkout.session.completed`, …
    #[serde(rename = "type")]
    pub kind: String,
    /// What it happened to.
    pub data: EventData,
}

/// An event's payload.
#[derive(Debug, Clone, Deserialize)]
pub struct EventData {
    /// The object, as it was when the event happened. Raw, because its shape
    /// depends on [`Event::kind`].
    pub object: serde_json::Value,
}

/// A caller carrying `key` the way Stripe takes it.
fn caller(key: &Secret) -> Caller {
    Caller::new(
        "Authorization",
        &Secret::new(format!("Bearer {}", key.expose())),
    )
}

/// Create a session.
pub fn create(key: &Secret, create: &Create<'_>) -> Result<Session, HttpError> {
    let form = create.form();
    let pairs: Vec<(&str, &str)> = form.iter().map(|(k, v)| (*k, v.as_str())).collect();
    caller(key).post_form(&format!("{BASE}/checkout/sessions"), &pairs)
}

/// Retrieve the session `id`.
pub fn retrieve(key: &Secret, id: &str) -> Result<Session, HttpError> {
    let id = crate::api::http::encoded(id);
    caller(key).get(&format!("{BASE}/checkout/sessions/{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_names_one_priced_line_and_no_payment_method() {
        let form = Create {
            currency: "brl",
            unit_amount: 5_000,
            name: "Credits",
            success_url: "https://example.com/credits/return?session={CHECKOUT_SESSION_ID}",
            cancel_url: "https://example.com/spending",
            expires_at: 1_800_000_000,
            client_reference_id: "7",
        }
        .form();
        let get = |name: &str| {
            form.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("mode"), Some("payment"));
        assert_eq!(get("line_items[0][price_data][unit_amount]"), Some("5000"));
        assert_eq!(get("line_items[0][price_data][currency]"), Some("brl"));
        assert_eq!(get("expires_at"), Some("1800000000"));
        assert!(form.iter().all(|(k, _)| !k.starts_with("payment_method")));
    }

    #[test]
    fn a_session_reads_from_stripes_example() {
        let session: Session = serde_json::from_str(
            r#"{"id": "cs_test_a1", "object": "checkout.session", "amount_total": 5000,
                "currency": "brl", "payment_status": "paid", "status": "complete",
                "url": null, "metadata": {}}"#,
        )
        .unwrap();
        assert_eq!(session.id, "cs_test_a1");
        assert_eq!(session.amount_total, Some(5_000));
        assert_eq!(session.status.as_deref(), Some("complete"));
        assert_eq!(session.url, None);
    }
}
