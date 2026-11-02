//! Which purchase a payment intent belongs to (#1637).
//!
//! A refund or a dispute names a charge's payment intent, never the Checkout
//! Session a `payments` row is keyed on. So the intent is kept on the row
//! when its session is credited (`keep`), and a row credited before that
//! was done is found by its session the first time an event names an
//! intent no row has (`owner`): each paid row with none is retrieved from
//! Stripe once, newest first, and given the intent its session names.

use scorsese_providers::api::http::HttpError;
use scorsese_providers::payments::Session;
use sqlx::postgres::PgPool;

use super::{PaymentError, Payments};
use crate::db::{self, Tx, UserId};

/// How many paid rows never asked about are retrieved from Stripe before
/// the intent is looked for again.
pub const BACKFILL_AT_ONCE: i64 = 20;

/// Keep the payment intent `session` names on payment `id`, unless one is
/// kept already.
pub(super) async fn keep(tx: &mut Tx, id: i64, session: &Session) -> Result<(), sqlx::Error> {
    let Some(intent) = &session.payment_intent else {
        return Ok(());
    };
    sqlx::query("UPDATE payments SET payment_intent = $2 WHERE id = $1 AND payment_intent IS NULL")
        .bind(id)
        .bind(intent)
        .execute(&mut **tx)
        .await
        .map(drop)
}

/// Who owns the purchase paid through `intent`, if anybody here does — the
/// one cross-user read a webhook needs, since it carries no user. When no
/// row has it, [`BACKFILL_AT_ONCE`] of the paid rows never asked about are
/// retrieved from Stripe, newest first: each is given the intent its
/// session names and marked asked, so it is never retrieved again. If
/// that does not find it and rows are left to ask about, or Stripe fails
/// to answer, it is [`PaymentError::Backfilling`], which the webhook
/// answers with an error status, so Stripe sends the event again and the
/// next delivery asks about the next rows — never one long request, and
/// never an intent taken as nobody's before every row was asked.
pub(super) async fn owner(
    pool: &PgPool,
    payments: &Payments,
    intent: &str,
) -> Result<Option<UserId>, PaymentError> {
    if let Some(owner) = find(pool, intent).await? {
        return Ok(Some(owner));
    }
    let mut tx = db::privileged(pool).await?;
    let unasked: Vec<(i64, String)> = sqlx::query_as(
        "SELECT user_id, stripe_session FROM payments
         WHERE state = 'paid' AND payment_intent IS NULL AND NOT intent_asked
         ORDER BY id DESC LIMIT $1",
    )
    .bind(BACKFILL_AT_ONCE)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    if unasked.is_empty() {
        return Ok(None);
    }
    for (user, id) in unasked {
        let found = intent_of(payments, &id).await?;
        let mut tx = db::scoped(pool, UserId::from_row(user)).await?;
        sqlx::query(
            "UPDATE payments SET payment_intent = $2, intent_asked = true
             WHERE stripe_session = $1 AND payment_intent IS NULL",
        )
        .bind(&id)
        .bind(&found)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
    }
    if let Some(owner) = find(pool, intent).await? {
        return Ok(Some(owner));
    }
    let mut tx = db::privileged(pool).await?;
    let left: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM payments
                        WHERE state = 'paid' AND payment_intent IS NULL AND NOT intent_asked)",
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    if left {
        return Err(PaymentError::Backfilling);
    }
    Ok(None)
}

/// The payment intent Stripe's session `id` names: `None` when it names
/// none, or Stripe has no such session (one made under another key). An
/// error when Stripe could not be asked.
async fn intent_of(payments: &Payments, id: &str) -> Result<Option<String>, PaymentError> {
    let checkout = payments.checkout.clone();
    let asked = id.to_owned();
    let read = tokio::task::spawn_blocking(move || checkout.retrieve(&asked))
        .await
        .map_err(|_| PaymentError::Interrupted)?;
    match read {
        Ok(session) if session.id == id => Ok(session.payment_intent),
        Ok(other) => {
            tracing::warn!("Stripe answered session {} when asked for {id}", other.id);
            Ok(None)
        }
        Err(HttpError::Refused { status: 404, .. }) => {
            tracing::warn!("Stripe has no session {id}, so its payment intent is unknown");
            Ok(None)
        }
        Err(error) => {
            tracing::warn!("session {id}'s payment intent could not be read: {error}");
            Err(PaymentError::Backfilling)
        }
    }
}

/// The owner of the row paid through `intent`, as it is stored.
async fn find(pool: &PgPool, intent: &str) -> Result<Option<UserId>, PaymentError> {
    let mut tx = db::privileged(pool).await?;
    let owner: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM payments WHERE payment_intent = $1")
            .bind(intent)
            .fetch_optional(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(owner.map(UserId::from_row))
}
