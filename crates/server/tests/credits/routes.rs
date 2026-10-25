//! `GET /api/credits` and `GET /api/credits/history`, over HTTP as the web
//! app reads them.

use scorsese_server::accounts::tokens;
use scorsese_server::credits::allowance::{self, Allowance};
use scorsese_server::credits::generations::Answer;
use sqlx::postgres::PgPool;

use super::common::{self, request};
use super::{SHOT_PRICE, account, finish, fund, shot, start};

#[sqlx::test]
async fn a_member_reads_their_balance_and_history(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let paid = start(&pool, ana, &shot(Some(4))).await.unwrap();
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();
    let token = tokens::issue(&pool, ana, "script").await.unwrap().token;
    let bearer = format!("Authorization: Bearer {token}");

    let credits = request(address, "GET", "/api/credits", &[bearer.as_str()], None).await;
    assert_eq!(credits.status, 200, "{}", credits.body);
    assert_eq!(credits.json()["balance_micros"], 10_000_000 - SHOT_PRICE);
    assert!(credits.json()["rate"].is_null());

    let path = "/api/credits/history?project=4&kind=veo_shot&since=2026-01-01";
    let history = request(address, "GET", path, &[bearer.as_str()], None).await;
    assert_eq!(history.status, 200, "{}", history.body);
    let history = history.json();
    assert_eq!(history["matched"], 1);
    assert_eq!(history["rows"][0]["status"], "charged");
    assert_eq!(history["rows"][0]["amount_micros"], -SHOT_PRICE);
    assert_eq!(history["rows"][0]["detail"]["seconds"], 8);
}

#[sqlx::test]
async fn a_bad_filter_is_a_bad_request_and_a_stranger_is_turned_away(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = account(&pool, "ana@example.com").await;
    let token = tokens::issue(&pool, ana, "script").await.unwrap().token;
    let bearer = format!("Authorization: Bearer {token}");

    let bad = request(
        address,
        "GET",
        "/api/credits/history?kind=all",
        &[bearer.as_str()],
        None,
    )
    .await;
    assert_eq!(bad.status, 400, "{}", bad.body);
    assert!(bad.json()["error"].as_str().unwrap().contains("veo_shot"));

    for path in ["/api/credits", "/api/credits/history"] {
        assert_eq!(common::get(address, path).await.0, 401, "{path}");
    }
}

/// The free allowance left today, and when it renews, beside the balance; a
/// spend it paid for shows its free part in the history (#1530).
#[sqlx::test]
async fn a_member_reads_what_is_free_today(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    allowance::set(&pool, Allowance::DEFAULT).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    let settings = serde_json::json!({});
    let paid = start(&pool, ana, &super::allowance::line(100, &settings))
        .await
        .unwrap();
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();
    let token = tokens::issue(&pool, ana, "script").await.unwrap().token;
    let bearer = format!("Authorization: Bearer {token}");

    let credits = request(address, "GET", "/api/credits", &[bearer.as_str()], None).await;
    assert_eq!(credits.status, 200, "{}", credits.body);
    let credits = credits.json();
    assert_eq!(credits["balance_micros"], 0);
    assert_eq!(credits["free_micros"], 4_000_000 - 1_800_000);
    assert!(credits["free_renews_at"].as_i64().unwrap() > 0, "{credits}");

    let history = request(
        address,
        "GET",
        "/api/credits/history",
        &[bearer.as_str()],
        None,
    )
    .await;
    let history = history.json();
    assert_eq!(history["rows"][0]["amount_micros"], 0);
    assert_eq!(history["rows"][0]["free_micros"], 1_800_000);
}
