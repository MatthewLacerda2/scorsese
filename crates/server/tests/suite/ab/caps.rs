//! The caps on a public write: a visitor's day of events, an address's day
//! of rows.

use scorsese_server::ab::{EVENTS_PER_BATCH, EVENTS_PER_VISITOR_PER_DAY};
use scorsese_server::http::ab::ROWS_PER_ADDRESS_PER_DAY;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::common::{self, request};
use super::events::batch;
use super::{ANA, PATH, events};

#[sqlx::test]
async fn a_visitor_keeps_at_most_a_days_events(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let views = json!(vec![json!({ "kind": "view" }); EVENTS_PER_BATCH]);
    let batches = EVENTS_PER_VISITOR_PER_DAY as usize / EVENTS_PER_BATCH + 2;
    for at in 0..batches {
        // Each from its own address, so only the visitor's cap can bite.
        let from = format!("X-Scorsese-Client: 198.51.100.{at}");
        let body = batch(ANA, "a", views.clone());
        assert_eq!(
            request(address, "POST", PATH, &[from.as_str()], Some(&body))
                .await
                .status,
            204
        );
    }
    assert_eq!(events(&pool).await.len() as i64, EVENTS_PER_VISITOR_PER_DAY);
}

#[sqlx::test]
async fn an_address_writes_at_most_a_days_rows(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let from = "X-Scorsese-Client: 203.0.113.7";
    let mut sent = 0;
    for at in 0.. {
        // A new visitor each time, so only the address's cap can bite.
        let visitor = format!("0b7f6a3e-9c1d-4e2f-8a5b-{at:012x}");
        let body = batch(&visitor, "a", json!([{ "kind": "view" }]));
        assert_eq!(
            request(address, "POST", PATH, &[from], Some(&body))
                .await
                .status,
            204
        );
        sent += 3;
        if sent > ROWS_PER_ADDRESS_PER_DAY + 30 {
            break;
        }
    }
    let visitors: i64 = sqlx::query_scalar("SELECT count(*) FROM ab_visitors")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(visitors, i64::from(ROWS_PER_ADDRESS_PER_DAY / 3));
    let elsewhere = batch(ANA, "a", json!([{ "kind": "view" }]));
    request(
        address,
        "POST",
        PATH,
        &["X-Scorsese-Client: 203.0.113.8"],
        Some(&elsewhere),
    )
    .await;
    assert_eq!(events(&pool).await.len() as i64, visitors + 1);
}
