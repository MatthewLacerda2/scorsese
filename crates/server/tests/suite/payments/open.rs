//! One open Checkout Session per member (#1654): a purchase started while
//! one is open is that one, and both routes that reach Stripe are limited.

use std::sync::Arc;

use sqlx::postgres::PgPool;

use super::{Fake, bought, member, serve};

#[sqlx::test]
async fn a_second_purchase_while_one_is_open_is_that_one(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("5")).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    assert_eq!(bought(address, &bearer, 5_000).await, "cs_test_1");
    assert_eq!(bought(address, &bearer, 5_000).await, "cs_test_1");
    assert_eq!(fake.created(), 1, "one Checkout Session, not two");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM payments")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}
