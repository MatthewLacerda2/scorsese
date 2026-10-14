//! Reading a Stripe webhook: is it Stripe's, and what does it say.
//!
//! Stripe signs each delivery (<https://docs.stripe.com/webhooks>, *Verify
//! webhook signatures manually*, read 2026-10-14). The `Stripe-Signature`
//! header is `t=<seconds>,v1=<hex>[,v1=<hex>…][,v0=<hex>]`; the signed
//! payload is the timestamp, a `.`, and the **raw** body; the signature is
//! its HMAC-SHA256 under the endpoint's signing secret (`whsec_…`, used as
//! it is). Only `v1` counts — the others are ignored "to prevent downgrade
//! attacks" — any one matching is enough (a rolled secret signs twice for a
//! day), compared in constant time, and a timestamp more than
//! [`TOLERANCE_SECONDS`] from now is refused as a replay.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::api::stripe::{Event, Session};
use crate::credentials::Secret;

/// How far a delivery's timestamp may be from now: five minutes, the
/// tolerance Stripe's own libraries default to.
pub const TOLERANCE_SECONDS: i64 = 300;

/// Why a delivery was not taken as Stripe's.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WebhookError {
    /// The header is missing, or carries no timestamp or no `v1`.
    #[error("the Stripe-Signature header is missing or malformed")]
    Malformed,
    /// No `v1` signature matches the body.
    #[error("no signature matches the body")]
    Mismatch,
    /// The signature matches, and was made too long ago (or ahead).
    #[error("the signature's timestamp is more than {TOLERANCE_SECONDS} seconds from now")]
    Stale,
    /// The signed body is not an event.
    #[error("the event is unreadable: {0}")]
    Unreadable(String),
}

/// `body` as an event, if `header` signs it under `secret` within the
/// tolerance of `now` (seconds since the epoch).
pub fn verify(
    header: Option<&str>,
    body: &[u8],
    secret: &Secret,
    now: i64,
) -> Result<Event, WebhookError> {
    let header = header.ok_or(WebhookError::Malformed)?;
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for element in header.split(',') {
        match element.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
            Some(("v1", value)) => signatures.push(value),
            _ => {}
        }
    }
    let (Some(timestamp), false) = (timestamp, signatures.is_empty()) else {
        return Err(WebhookError::Malformed);
    };
    let expected = sign(secret, timestamp, body);
    if !signatures
        .iter()
        .any(|signature| matches(&expected, signature))
    {
        return Err(WebhookError::Mismatch);
    }
    if (now - timestamp).abs() > TOLERANCE_SECONDS {
        return Err(WebhookError::Stale);
    }
    serde_json::from_slice(body).map_err(|error| WebhookError::Unreadable(error.to_string()))
}

/// The `v1` signature of `body` at `timestamp`, as lowercase hex — what
/// Stripe sends, and what a test signs its own deliveries with.
pub fn sign(secret: &Secret, timestamp: i64, body: &[u8]) -> String {
    // HMAC takes a key of any length, so this cannot fail.
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.expose().as_bytes())
        .unwrap_or_else(|_| unreachable!("HMAC accepts any key length"));
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Whether `received` is `expected`, compared without stopping at the first
/// difference, so how long it takes says nothing about where they differ.
fn matches(expected: &str, received: &str) -> bool {
    expected.len() == received.len()
        && expected
            .bytes()
            .zip(received.bytes())
            .fold(0u8, |differ, (a, b)| differ | (a ^ b))
            == 0
}

/// What an event means for a payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Happened {
    /// A session completed, or its delayed payment succeeded: credit it if
    /// it is paid.
    Settled(Session),
    /// A session expired unpaid.
    Expired(Session),
    /// Anything else, which is answered and ignored.
    Other,
}

impl Happened {
    /// What `event` means.
    pub fn of(event: &Event) -> Result<Self, WebhookError> {
        let session = || {
            serde_json::from_value::<Session>(event.data.object.clone())
                .map_err(|error| WebhookError::Unreadable(error.to_string()))
        };
        Ok(match event.kind.as_str() {
            "checkout.session.completed" | "checkout.session.async_payment_succeeded" => {
                Self::Settled(session()?)
            }
            "checkout.session.expired" => Self::Expired(session()?),
            _ => Self::Other,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &[u8] = br#"{"id": "evt_1", "type": "checkout.session.completed",
        "data": {"object": {"id": "cs_test_1", "payment_status": "paid",
        "status": "complete", "amount_total": 5000, "currency": "brl"}}}"#;

    fn secret() -> Secret {
        Secret::new("whsec_test")
    }

    fn header(timestamp: i64, body: &[u8]) -> String {
        format!(
            "t={timestamp},v1={},v0=ignored",
            sign(&secret(), timestamp, body)
        )
    }

    #[test]
    fn a_signature_matches_stripes_documented_scheme() {
        // HMAC-SHA256("whsec_test", "1700000000.{}"), computed independently
        // with `openssl dgst -sha256 -hmac whsec_test`.
        assert_eq!(
            sign(&secret(), 1_700_000_000, b"{}"),
            "35495024f4ef3f94e5a93e22221544c4b75e9a42300cd965ab81cb85cd994e91"
        );
    }

    #[test]
    fn a_signed_fresh_delivery_is_an_event() {
        let event = verify(Some(&header(1_000, BODY)), BODY, &secret(), 1_100).unwrap();
        assert_eq!(event.id, "evt_1");
        let Happened::Settled(session) = Happened::of(&event).unwrap() else {
            panic!("a completed session settles");
        };
        assert_eq!(session.id, "cs_test_1");
    }

    #[test]
    fn a_rolled_secret_signs_twice_and_either_is_enough() {
        let header = format!("t=1000,v1=00,v1={}", sign(&secret(), 1_000, BODY));
        assert!(verify(Some(&header), BODY, &secret(), 1_000).is_ok());
    }

    #[test]
    fn a_tampered_body_or_another_secret_is_a_mismatch() {
        let mut tampered = BODY.to_vec();
        tampered[10] = b'X';
        let signed = header(1_000, BODY);
        assert_eq!(
            verify(Some(&signed), &tampered, &secret(), 1_000).unwrap_err(),
            WebhookError::Mismatch
        );
        let other = Secret::new("whsec_other");
        assert_eq!(
            verify(Some(&signed), BODY, &other, 1_000).unwrap_err(),
            WebhookError::Mismatch
        );
        // Only v1 counts: a v0 that matches is not enough.
        let v0 = format!("t=1000,v0={}", sign(&secret(), 1_000, BODY));
        assert_eq!(
            verify(Some(&v0), BODY, &secret(), 1_000).unwrap_err(),
            WebhookError::Malformed
        );
    }

    #[test]
    fn a_stale_or_future_delivery_is_refused_past_the_tolerance() {
        let signed = header(1_000, BODY);
        assert!(verify(Some(&signed), BODY, &secret(), 1_000 + TOLERANCE_SECONDS).is_ok());
        for now in [1_001 + TOLERANCE_SECONDS, 999 - TOLERANCE_SECONDS] {
            assert_eq!(
                verify(Some(&signed), BODY, &secret(), now).unwrap_err(),
                WebhookError::Stale,
                "{now}"
            );
        }
    }

    #[test]
    fn a_missing_or_malformed_header_is_malformed() {
        for header in [
            None,
            Some(""),
            Some("v1=ab"),
            Some("t=x,v1=ab"),
            Some("t=1000"),
        ] {
            assert_eq!(
                verify(header, BODY, &secret(), 1_000).unwrap_err(),
                WebhookError::Malformed,
                "{header:?}"
            );
        }
    }

    #[test]
    fn expiry_and_other_events_are_told_apart() {
        let expired: Event = serde_json::from_str(
            r#"{"id": "evt_2", "type": "checkout.session.expired",
                "data": {"object": {"id": "cs_test_1", "payment_status": "unpaid",
                "status": "expired"}}}"#,
        )
        .unwrap();
        assert!(matches!(Happened::of(&expired), Ok(Happened::Expired(_))));
        let other: Event = serde_json::from_str(
            r#"{"id": "evt_3", "type": "charge.refunded", "data": {"object": {"id": "ch_1"}}}"#,
        )
        .unwrap();
        assert_eq!(Happened::of(&other).unwrap(), Happened::Other);
    }
}
