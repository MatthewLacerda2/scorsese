//! A dispute's debt (#1637): while a dispute leaves the paid balance below
//! zero, the assistant runs on nothing, the free allowance included, and
//! says why.

use scorsese_server::credits::allowance::{self, Allowance};
use scorsese_server::db::{self, UserId};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, member, project, send};

/// A purchase of `user`'s, paid, and disputed for `micros` more than was
/// left of it, written as the webhook writes one.
async fn disputed(pool: &PgPool, user: UserId, micros: i64) {
    let mut tx = db::scoped(pool, user).await.expect("the test setup holds");
    let payment: i64 = sqlx::query_scalar(
        "INSERT INTO payments (user_id, stripe_session, brl_centavos, credit_micros,
                               brl_per_usd, rate_source, state, paid_at)
         VALUES (member_id(), 'cs_test_1', 5000, 10000000, '5.00', 'awesomeapi', 'paid', now())
         RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("the test setup holds");
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, memo, payment_id)
         VALUES (member_id(), 'dispute', $1, 'disputed', $2)",
    )
    .bind(-micros)
    .bind(payment)
    .execute(&mut *tx)
    .await
    .expect("the ledger takes a dispute");
    tx.commit().await.expect("the test setup holds");
}

#[sqlx::test]
async fn a_disputed_account_is_refused_with_the_reason_even_with_free_credit(pool: PgPool) {
    allowance::set(&pool, Allowance::DEFAULT).await.unwrap();
    let script = Script::new(vec![answers("never")]);
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 0).await;
    disputed(&pool, ana, 4_200_000).await;
    let id = project(&pool, ana, json!([])).await;
    let refused = send(address, &who, id, "make an intro").await;
    assert_eq!(refused.status, 402, "{}", refused.body);
    assert!(
        refused.body.contains("because a payment was disputed"),
        "{}",
        refused.body
    );
    assert!(script.requests().is_empty());
}
