//! A failed generation gives its free part back to the allowance, never to
//! the paid balance.

use scorsese_server::credits::allowance;
use scorsese_server::credits::generations::Answer;
use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::{account, balance, finish, fund, start};
use super::{FOUR_A_DAY, free, line};

#[sqlx::test]
async fn a_failed_generation_gives_its_free_part_back_to_the_allowance(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 1).await;
    let settings = json!({});
    // $5.40: $4 free and $1.40 — more than the $1 paid, so refused...
    assert!(start(&pool, ana, &line(300, &settings)).await.is_err());
    // ...and $4.50: $4 free, 50 cents paid.
    let paid = start(&pool, ana, &line(250, &settings)).await.unwrap();
    assert_eq!(balance(&pool, ana).await, 500_000);
    assert_eq!(free(&pool, ana).await, 0);
    finish(&pool, ana, paid, &Answer::Failed("refused".into()))
        .await
        .unwrap();
    assert_eq!(
        free(&pool, ana).await,
        4_000_000,
        "the free part, back to free"
    );
    assert_eq!(
        balance(&pool, ana).await,
        1_000_000,
        "the paid part, back to paid"
    );
}

/// Failing generations on purpose cannot turn free credit into paid credit.
#[sqlx::test]
async fn free_credit_never_lands_in_the_paid_balance(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    let settings = json!({});
    for _ in 0..5 {
        let paid = start(&pool, ana, &line(200, &settings)).await.unwrap();
        finish(&pool, ana, paid, &Answer::Failed("on purpose".into()))
            .await
            .unwrap();
    }
    assert_eq!(balance(&pool, ana).await, 0);
    assert_eq!(free(&pool, ana).await, 4_000_000);

    // Each grant is given back at most once: the index says so.
    let paid = start(&pool, ana, &line(200, &settings)).await.unwrap();
    finish(&pool, ana, paid, &Answer::Failed("once".into()))
        .await
        .unwrap();
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let twice = sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, settles, memo)
         VALUES (member_id(), 'allowance', -3600000, $1, 'again')",
    )
    .bind(paid.reservation.entry)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert!(
        twice
            .to_string()
            .contains("credit_entries_one_allowance_back"),
        "{twice}"
    );
}
