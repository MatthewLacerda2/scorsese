//! One open Checkout Session per member (#1654).
//!
//! A purchase started while the member's last session is still open at
//! Stripe is that session when it asks for the same reais at the same rate
//! and bonus — a double click, a reload, the back button — and otherwise
//! expires it at Stripe before a new one is made. So each member has at most
//! one session Stripe would take payment on, and starting purchases over and
//! over makes no more of them. Two starts racing are serialized by a
//! transaction-scoped advisory lock per member, held until the new row is
//! written.

use scorsese_providers::api::http::HttpError;
use scorsese_providers::payments::{Checkout, Session};

use super::settle::settle;
use super::{Begun, EXPIRES_AFTER_SECONDS, PaymentError, Payments, Quote};
use crate::db::Tx;

/// The advisory lock's first key: purchases being started. Two-key advisory
/// locks are a key space of their own, apart from the job worker's one-key
/// lock; the second key is the member.
const STARTING: i32 = 1654;

/// Hold, until `tx` ends, the lock on starting a purchase for its member.
pub(super) async fn lock(tx: &mut Tx) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, (member_id() % 2147483647)::int)")
        .bind(STARTING)
        .execute(&mut **tx)
        .await
        .map(drop)
}

/// The member's newest payment still `open` and young enough that its
/// session may still take payment.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct Open {
    stripe_session: String,
    brl_centavos: i64,
    credit_micros: i64,
    bonus_micros: i64,
    brl_per_usd: String,
}

impl Open {
    /// The member's, read in `tx`, if there is one.
    pub(super) async fn newest(tx: &mut Tx) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(
            "SELECT stripe_session, brl_centavos, credit_micros, bonus_micros, brl_per_usd
             FROM payments
             WHERE state = 'open' AND created_at > now() - make_interval(secs => $1)
             ORDER BY id DESC LIMIT 1",
        )
        .bind(EXPIRES_AFTER_SECONDS as f64)
        .fetch_optional(&mut **tx)
        .await
    }

    /// Whether it was started for exactly what `quote` sells.
    fn sells(&self, quote: &Quote) -> bool {
        self.brl_centavos == quote.brl_centavos
            && self.credit_micros == quote.credit_micros
            && self.bonus_micros == quote.bonus_micros
            && self.brl_per_usd == quote.brl_per_usd
    }

    /// This session, when Stripe still takes payment on it for exactly what
    /// `quote` sells; otherwise `None`, once it is expired at Stripe if it
    /// was open, and settled here either way.
    pub(super) async fn reuse_or_close(
        self,
        pool: &sqlx::PgPool,
        payments: &Payments,
        quote: &Quote,
    ) -> Result<Option<Begun>, PaymentError> {
        let current = call(payments, &self.stripe_session, |checkout, id| {
            checkout.retrieve(id)
        })
        .await?;
        let open = current.status.as_deref() == Some("open");
        if open
            && self.sells(quote)
            && let Some(url) = current.url
        {
            return Ok(Some(Begun {
                url,
                session: current.id,
                quote: quote.clone(),
            }));
        }
        let current = if open {
            call(payments, &self.stripe_session, |checkout, id| {
                checkout.expire(id)
            })
            .await?
        } else {
            current
        };
        settle(pool, &current).await?;
        Ok(None)
    }
}

/// One of Stripe's calls on a session, as [`call`] makes it.
type SessionCall = fn(&dyn Checkout, &str) -> Result<Session, HttpError>;

/// `call` made on Stripe with session `id`, off the async runtime.
async fn call(payments: &Payments, id: &str, call: SessionCall) -> Result<Session, PaymentError> {
    let checkout = payments.checkout.clone();
    let id = id.to_owned();
    let session = tokio::task::spawn_blocking(move || call(checkout.as_ref(), &id))
        .await
        .map_err(|_| PaymentError::Interrupted)??;
    Ok(session)
}
