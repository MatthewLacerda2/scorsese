//! One open Checkout Session per member (#1654).
//!
//! A purchase started while the member's last session is still open at
//! Stripe is that session when it asks for the same reais at the same rate,
//! source and bonus and has at least [`REUSED_WITHIN_SECONDS`]' worth of its
//! life left — a double click, a reload, the back button — and otherwise
//! expires it at Stripe before a new one is made. So each member has at most
//! one session Stripe would take payment on, and starting purchases over and
//! over makes no more of them.
//!
//! Two starts racing for one member are serialized by [`Starting`], a lock
//! in this process held from reading the open session to writing the new
//! one. It holds no database connection: each read and write is its own short
//! transaction, so a slow Stripe never keeps a connection from the pool.

use std::sync::Arc;

use scorsese_providers::api::http::HttpError;
use scorsese_providers::payments::{Checkout, Session};
use sqlx::postgres::PgPool;
use tokio::sync::{Mutex, MutexGuard};

use super::settle::settle;
use super::{Begun, EXPIRES_AFTER_SECONDS, PaymentError, Payments, Quote};
use crate::db::{self, UserId};

/// How many locks members share. Two members on one only wait for each
/// other's start; the set stays this size however many members there are.
const LANES: usize = 64;

/// How young an open session must be to be handed back: ten minutes, so the
/// person sent to it has at least twenty of its thirty-one to pay in.
pub const REUSED_WITHIN_SECONDS: i64 = 10 * 60;

/// Purchases being started, one at a time per member.
#[derive(Clone)]
pub struct Starting(Arc<[Mutex<()>]>);

impl Default for Starting {
    fn default() -> Self {
        Self((0..LANES).map(|_| Mutex::new(())).collect())
    }
}

impl Starting {
    /// Wait for, and hold, `user`'s turn to start a purchase.
    pub(super) async fn lock(&self, user: UserId) -> MutexGuard<'_, ()> {
        let lane = usize::try_from(user.get().rem_euclid(LANES as i64)).unwrap_or(0);
        self.0[lane].lock().await
    }
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
    rate_source: String,
    /// Whether it is young enough to hand back.
    fresh: bool,
}

impl Open {
    /// `user`'s, if there is one.
    pub(super) async fn newest(pool: &PgPool, user: UserId) -> Result<Option<Self>, sqlx::Error> {
        let mut tx = db::scoped(pool, user).await?;
        let open = sqlx::query_as(
            "SELECT stripe_session, brl_centavos, credit_micros, bonus_micros, brl_per_usd,
                    rate_source, created_at > now() - make_interval(secs => $2) AS fresh
             FROM payments
             WHERE state = 'open' AND created_at > now() - make_interval(secs => $1)
             ORDER BY id DESC LIMIT 1",
        )
        .bind(EXPIRES_AFTER_SECONDS as f64)
        .bind(REUSED_WITHIN_SECONDS as f64)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(open)
    }

    /// Whether it may be handed back for what `quote` sells.
    fn sells(&self, quote: &Quote) -> bool {
        self.fresh
            && self.brl_centavos == quote.brl_centavos
            && self.credit_micros == quote.credit_micros
            && self.bonus_micros == quote.bonus_micros
            && self.brl_per_usd == quote.brl_per_usd
            && self.rate_source == quote.rate_source
    }

    /// This session, when Stripe still takes payment on it and it may be
    /// handed back for `quote`; otherwise `None`, once it is expired at
    /// Stripe if it was open, and settled here either way.
    pub(super) async fn reuse_or_close(
        self,
        pool: &PgPool,
        payments: &Payments,
        quote: &Quote,
    ) -> Result<Option<Begun>, PaymentError> {
        let retrieve = |checkout: &dyn Checkout, id: &str| checkout.retrieve(id);
        let current = call(payments, &self.stripe_session, retrieve).await?;
        let open = is_open(&current);
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
            let expire = |checkout: &dyn Checkout, id: &str| checkout.expire(id);
            match call(payments, &self.stripe_session, expire).await {
                Ok(expired) => expired,
                // Paid or expired since it was read: Stripe refuses to expire
                // it, and it is settled as it is now.
                Err(refused) => {
                    let now = call(payments, &self.stripe_session, retrieve).await?;
                    if is_open(&now) {
                        return Err(refused);
                    }
                    now
                }
            }
        } else {
            current
        };
        settle(pool, &current).await?;
        Ok(None)
    }
}

/// Whether Stripe would still take payment on `session`.
fn is_open(session: &Session) -> bool {
    session.status.as_deref() == Some("open")
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
