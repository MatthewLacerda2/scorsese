//! Withdrawing a purchase within its 7 days (#1537): what is left of it,
//! its bonus included, taken back once, and the reais to refund.

use std::sync::Arc;

use scorsese_server::credits::command::{self, CreditCommand};
use scorsese_server::credits::ledger::{self, Link};
use scorsese_server::db::{self, UserId};
use scorsese_server::payments::{settle, withdraw};
use sqlx::postgres::PgPool;

use super::bonus::{EIGHTY, entries, serve_bonus};
use super::{Fake, balance, bought, member};

/// Buy and pay R$ 80 as a fresh `ana`; her id and the session.
async fn eighty_paid(pool: &PgPool) -> (UserId, String) {
    let fake = Arc::new(Fake::default());
    let address = serve_bonus(pool, fake.clone(), "4.99").await;
    let (ana, bearer) = member(pool, "ana@example.com").await;
    let session = bought(address, &bearer, 8_000).await;
    let paid = settle::settle(pool, &fake.pay(&session)).await;
    paid.expect("a paid session settles");
    (ana, session)
}

#[sqlx::test]
async fn a_withdrawal_takes_back_what_is_left_bonus_included_once(pool: PgPool) {
    let (ana, session) = eighty_paid(&pool).await;
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let withdrawn = withdraw::withdraw(&mut tx, &session, None).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(withdrawn.taken_micros, EIGHTY.0 + EIGHTY.1);
    assert_eq!(withdrawn.bonus_micros, EIGHTY.1);
    assert_eq!(withdrawn.refund_centavos, 8_000);
    assert_eq!(balance(&pool, ana).await, 0);

    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let again = withdraw::withdraw(&mut tx, &session, None).await;
    assert!(again.is_err(), "a purchase is withdrawn once: {again:?}");
}

#[sqlx::test]
async fn a_withdrawal_after_spending_takes_back_its_share_of_the_bonus(pool: PgPool) {
    let (ana, session) = eighty_paid(&pool).await;
    let half = (EIGHTY.0 + EIGHTY.1) / 2;
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let held = ledger::reserve(&mut tx, half, "a still", Link::default())
        .await
        .unwrap();
    ledger::charge(&mut tx, held, half).await.unwrap();
    let withdrawn = withdraw::withdraw(&mut tx, &session, None).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(withdrawn.taken_micros, EIGHTY.0 + EIGHTY.1 - half);
    assert_eq!(withdrawn.bonus_micros, EIGHTY.1 / 2);
    assert_eq!(withdrawn.refund_centavos, 4_000);
    assert_eq!(balance(&pool, ana).await, 0);
    assert_eq!(entries(&pool, ana, "withdrawal").await, 1);
}

#[sqlx::test]
async fn a_purchase_past_its_seven_days_is_not_withdrawn(pool: PgPool) {
    let (ana, session) = eighty_paid(&pool).await;
    let mut operator = db::privileged(&pool).await.unwrap();
    sqlx::query("UPDATE payments SET paid_at = now() - interval '8 days'")
        .execute(&mut *operator)
        .await
        .unwrap();
    operator.commit().await.unwrap();

    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let late = withdraw::withdraw(&mut tx, &session, None).await;
    assert!(late.is_err(), "{late:?}");
    drop(tx);
    assert_eq!(balance(&pool, ana).await, EIGHTY.0 + EIGHTY.1);

    // Asked for on its sixth day, it is withdrawn however late the operator is.
    let mut operator = db::privileged(&pool).await.unwrap();
    let asked: String = sqlx::query_scalar(
        "SELECT to_char((now() AT TIME ZONE 'America/Sao_Paulo')::date - 2, 'YYYY-MM-DD')",
    )
    .fetch_one(&mut *operator)
    .await
    .unwrap();
    drop(operator);
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let on_time = withdraw::withdraw(&mut tx, &session, Some(&asked)).await;
    assert_eq!(on_time.unwrap().taken_micros, EIGHTY.0 + EIGHTY.1);
}

#[sqlx::test]
async fn a_generation_still_waiting_holds_the_withdrawal_back(pool: PgPool) {
    let (ana, session) = eighty_paid(&pool).await;
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    ledger::reserve(&mut tx, 1_000_000, "a still", Link::default())
        .await
        .unwrap();
    let held = withdraw::withdraw(&mut tx, &session, None).await;
    assert!(held.is_err(), "{held:?}");
}

#[sqlx::test]
async fn the_ledger_refuses_a_bonus_without_its_payment(pool: PgPool) {
    let (ana, _) = eighty_paid(&pool).await;
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let orphan = sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, memo)
         VALUES (member_id(), 'bonus', 1, 'Bonus')",
    )
    .execute(&mut *tx)
    .await;
    assert!(orphan.is_err(), "a bonus names its payment");
}

#[sqlx::test]
async fn the_operators_withdraw_says_what_to_refund_in_reais(pool: PgPool) {
    let (_, session) = eighty_paid(&pool).await;
    let printed = command::run(
        &pool,
        CreditCommand::Withdraw {
            email: "ana@example.com".into(),
            session,
            requested: None,
        },
    )
    .await
    .unwrap();
    assert!(printed.contains("took back $17.63527"), "{printed}");
    assert!(printed.contains("($1.603206 of it bonus)"), "{printed}");
    assert!(
        printed.ends_with("refund R$ 80,00 of the R$ 80,00 paid, in Stripe's Dashboard"),
        "{printed}"
    );
}
