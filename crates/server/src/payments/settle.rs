//! Crediting a payment, exactly once, from whichever confirmation arrives.

use scorsese_providers::payments::{self, Session};
use serde::Serialize;
use sqlx::postgres::PgPool;

use super::{PaymentError, Payments};
use crate::credits::ledger;
use crate::db::{self, Tx, UserId};

/// What settling a session did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled {
    /// It was paid, and this call credited it.
    Credited,
    /// It was credited already, by the other way in.
    Already,
    /// It expired unpaid, and is now marked so.
    Expired,
    /// Nothing to do yet: unpaid, or paid with an amount that disagrees.
    Open,
    /// No payment has this session: another server's, or not ours at all.
    Unknown,
}

/// A payment as its owner reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct Payment {
    /// The Checkout Session's id.
    pub session: String,
    /// `open`, `paid` or `expired`.
    pub state: String,
    /// What was asked for, in centavos.
    pub brl_centavos: i64,
    /// What it credits, in micro-dollars.
    pub credit_micros: i64,
    /// The bonus it credits on top, in micro-dollars (#1537).
    pub bonus_micros: i64,
}

/// Credit `session`'s payment if Stripe says it is paid, or mark it expired
/// if it expired — once, however many times and from however many places
/// this is called. Both the webhook and the return page come here.
///
/// The payment's owner is found first, by session id, outside any user's
/// scope — the one cross-user read here, because a webhook carries no user.
/// Everything after runs scoped as that owner, holding the payment's row
/// lock, so a second caller waits and then finds it settled.
pub async fn settle(pool: &PgPool, session: &Session) -> Result<Settled, PaymentError> {
    let mut find = db::privileged(pool).await?;
    let owner: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM payments WHERE stripe_session = $1")
            .bind(&session.id)
            .fetch_optional(&mut *find)
            .await?;
    find.commit().await?;
    let Some(owner) = owner else {
        return Ok(Settled::Unknown);
    };

    let mut tx = db::scoped(pool, UserId::from_row(owner)).await?;
    let row: Option<(i64, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT id, state, brl_centavos, credit_micros, bonus_micros FROM payments
         WHERE stripe_session = $1 FOR UPDATE",
    )
    .bind(&session.id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((id, state, centavos, micros, bonus)) = row else {
        return Ok(Settled::Unknown);
    };
    let settled = match state.as_str() {
        "paid" => Settled::Already,
        "open" if payments::paid(session, centavos) => {
            ledger::top_up_paid(&mut tx, micros, id).await?;
            if bonus > 0 {
                ledger::bonus(&mut tx, bonus, id).await?;
            }
            mark(&mut tx, id, "paid").await?;
            Settled::Credited
        }
        "open" if payments::expired(session) => {
            mark(&mut tx, id, "expired").await?;
            Settled::Expired
        }
        "open" if session.payment_status == "paid" => {
            tracing::error!(
                "payment {id} ({}) was paid {:?} {:?}, not BRL {centavos} \
                 centavos as asked; not credited — look at it in Stripe's Dashboard",
                session.id,
                session.amount_total,
                session.currency
            );
            Settled::Open
        }
        _ => Settled::Open,
    };
    tx.commit().await?;
    Ok(settled)
}

/// Move payment `id` to `state`, stamping when it was paid.
async fn mark(tx: &mut Tx, id: i64, state: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE payments
         SET state = $2, paid_at = CASE WHEN $2 = 'paid' THEN now() END
         WHERE id = $1",
    )
    .bind(id)
    .bind(state)
    .execute(&mut **tx)
    .await
    .map(drop)
}

/// `user`'s payment by `session`, brought up to date: when it is still open,
/// the session is retrieved from Stripe and settled first — the return page's
/// way in, which covers a webhook that is late or never comes.
pub async fn confirm(
    pool: &PgPool,
    payments: &Payments,
    user: UserId,
    session: &str,
) -> Result<Payment, PaymentError> {
    let payment = read(pool, user, session).await?;
    if payment.state != "open" {
        return Ok(payment);
    }
    let checkout = payments.checkout.clone();
    let id = payment.session.clone();
    let current = tokio::task::spawn_blocking(move || checkout.retrieve(&id))
        .await
        .map_err(|_| PaymentError::Interrupted)??;
    if current.id != payment.session {
        return Err(PaymentError::NotFound);
    }
    settle(pool, &current).await?;
    read(pool, user, session).await
}

/// `user`'s payment by `session`, as it is stored.
async fn read(pool: &PgPool, user: UserId, session: &str) -> Result<Payment, PaymentError> {
    let mut tx = db::scoped(pool, user).await?;
    let payment = sqlx::query_as(
        "SELECT stripe_session AS session, state, brl_centavos, credit_micros, bonus_micros
         FROM payments WHERE stripe_session = $1",
    )
    .bind(session)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    payment.ok_or(PaymentError::NotFound)
}
