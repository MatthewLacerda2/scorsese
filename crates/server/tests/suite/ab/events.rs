//! `POST /api/ab/events`: a batch is written once, its first version kept,
//! anything else dropped without a word.

use scorsese_server::ab::EVENTS_PER_BATCH;
use scorsese_server::accounts::users;
use scorsese_server::db;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::common::{self, request, send};
use super::{ANA, PATH, events};

pub(super) fn batch(visitor: &str, version: &str, events: Value) -> Value {
    json!({
        "visitor": visitor,
        "exposures": [{ "test": "heading", "version": version }],
        "events": events,
    })
}

#[sqlx::test]
async fn a_visit_is_recorded_where_it_came_from_and_keeps_its_first_version(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let place = ["X-Scorsese-Country: BR", "X-Scorsese-Region: SP"];
    let first = batch(
        ANA,
        "b",
        json!([{ "kind": "view" }, { "kind": "section", "value": 2 }]),
    );
    let taken = request(address, "POST", PATH, &place, Some(&first)).await;
    assert_eq!(taken.status, 204, "{}", taken.body);
    let later = batch(
        ANA,
        "a",
        json!([{ "kind": "end" }, { "kind": "time", "value": 41 }, { "kind": "prompt" }]),
    );
    let taken = request(
        address,
        "POST",
        PATH,
        &["X-Scorsese-Country: US"],
        Some(&later),
    )
    .await;
    assert_eq!(taken.status, 204);

    let visitor: (Option<String>, Option<String>, Option<i64>) =
        sqlx::query_as("SELECT country, region, user_id FROM ab_visitors WHERE code = $1::uuid")
            .bind(ANA)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(visitor, (Some("BR".into()), Some("SP".into()), None));
    let shown: Vec<(String, String)> = sqlx::query_as("SELECT test, version FROM ab_exposures")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(shown, [("heading".into(), "b".into())]);
    let kinds: Vec<(&str, i32)> = vec![
        ("view", 0),
        ("section", 2),
        ("end", 0),
        ("time", 41),
        ("prompt", 0),
    ];
    let recorded = events(&pool).await;
    let recorded: Vec<(&str, i32)> = recorded
        .iter()
        .map(|(kind, value)| (kind.as_str(), *value))
        .collect();
    assert_eq!(recorded, kinds);
}

#[sqlx::test]
async fn anything_not_of_our_shape_is_answered_and_dropped(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    for body in [
        batch("me", "a", json!([{ "kind": "view" }])),
        batch(ANA, "A", json!([{ "kind": "view" }])),
        batch(ANA, "a", json!([{ "kind": "time", "value": 99999 }])),
        batch(ANA, "a", json!([{ "kind": "click" }])),
        json!({ "visitor": ANA, "user_id": 1 }),
        batch(
            ANA,
            "a",
            json!(vec![json!({ "kind": "view" }); EVENTS_PER_BATCH + 1]),
        ),
    ] {
        let answered = request(address, "POST", PATH, &[], Some(&body)).await;
        assert_eq!(answered.status, 204, "{body}");
        assert!(answered.body.is_empty());
    }
    let heavy = send(
        address,
        "POST",
        PATH,
        &["Content-Type: application/json"],
        format!("{{\"visitor\":\"{}\"}}", "x".repeat(8192)).as_bytes(),
    )
    .await;
    assert_eq!(heavy.status, 204);
    let garbled = send(
        address,
        "POST",
        PATH,
        &["Content-Type: application/json"],
        b"{\"v",
    )
    .await;
    assert_eq!(garbled.status, 204);
    let visitors: i64 = sqlx::query_scalar("SELECT count(*) FROM ab_visitors")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(visitors, 0);
    assert!(events(&pool).await.is_empty());
}

#[sqlx::test]
async fn no_member_reads_them_and_a_deleted_account_lets_go(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let body = batch(ANA, "a", json!([{ "kind": "view" }]));
    request(address, "POST", PATH, &[], Some(&body)).await;
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    scorsese_server::ab::link(&pool, ANA, ana).await;

    let members = db::member_pool(&pool).await.unwrap();
    for query in [
        "SELECT count(*) FROM ab_visitors",
        "SELECT count(*) FROM ab_exposures",
        "SELECT count(*) FROM ab_events",
    ] {
        let mut tx = db::scoped(&members, ana).await.unwrap();
        let error = sqlx::query(query).execute(&mut *tx).await.unwrap_err();
        assert!(
            error.to_string().contains("permission denied"),
            "{query}: {error}"
        );
    }

    let linked: Option<i64> = sqlx::query_scalar("SELECT user_id FROM ab_visitors")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(linked, Some(ana.get()));
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(ana.get())
        .execute(&pool)
        .await
        .unwrap();
    let after: (Option<i64>, i64) =
        sqlx::query_as("SELECT user_id, (SELECT count(*) FROM ab_events) FROM ab_visitors")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after, (None, 1), "the link goes, the counts stay");
}
