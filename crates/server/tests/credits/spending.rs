//! Reserve, then settle: a success is charged, a failure is free, and nothing
//! is spent that the balance does not cover.

use scorsese_providers::prices::claude::{MODEL, Usage};
use scorsese_server::credits::generations::{Answer, Line, Request};
use scorsese_server::credits::ledger::{self, AssistantCall};
use scorsese_server::credits::{CreditError, price};
use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{SHOT_PRICE, account, balance, finish, fund, shot, start};

#[sqlx::test]
async fn a_shot_that_worked_is_charged_its_price_plus_ten_percent(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let paid = start(&pool, ana, &shot(Some(7))).await.unwrap();
    assert_eq!(paid.reservation.micros, SHOT_PRICE);
    assert_eq!(
        balance(&pool, ana).await,
        10_000_000 - SHOT_PRICE,
        "held while it runs"
    );

    let made = crate::common::hold(&pool, ana, &"c".repeat(64)).await;
    finish(&pool, ana, paid, &Answer::Worked(Some(made)))
        .await
        .unwrap();
    assert_eq!(balance(&pool, ana).await, 10_000_000 - SHOT_PRICE);

    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let (state, item, cost): (String, Option<i64>, i64) =
        sqlx::query_as("SELECT state, library_item_id, estimated_cost_micros FROM veo_generations")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(
        (state.as_str(), item, cost),
        ("generated", Some(made), 960_000)
    );
}

#[sqlx::test]
async fn a_shot_the_provider_failed_is_free(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let paid = start(&pool, ana, &shot(None)).await.unwrap();
    finish(&pool, ana, paid, &Answer::Failed("safety filter".into()))
        .await
        .unwrap();
    assert_eq!(balance(&pool, ana).await, 10_000_000);

    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let (state, error): (String, Option<String>) =
        sqlx::query_as("SELECT state, error FROM veo_generations")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(
        (state.as_str(), error.as_deref()),
        ("failed", Some("safety filter"))
    );
}

#[sqlx::test]
async fn a_generation_the_balance_cannot_cover_is_refused_and_leaves_nothing(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 1).await;
    let refused = start(&pool, ana, &shot(None)).await.unwrap_err();
    assert!(
        matches!(
            refused,
            CreditError::Insufficient {
                balance: 1_000_000,
                needed: SHOT_PRICE
            }
        ),
        "{refused:?}"
    );
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM veo_generations")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    assert_eq!(ledger::balance(&mut tx).await.unwrap(), 1_000_000);
}

#[sqlx::test]
async fn a_generation_is_settled_once(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let paid = start(&pool, ana, &shot(None)).await.unwrap();
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();
    let again = finish(&pool, ana, paid, &Answer::Failed("late".into())).await;
    assert!(matches!(again, Err(CreditError::NotOpen(_))), "{again:?}");
    assert_eq!(balance(&pool, ana).await, 10_000_000 - SHOT_PRICE);
}

/// Two shots started at once against enough for one: the lock on the user's
/// row decides them in turn, so exactly one is reserved.
#[sqlx::test]
async fn two_spends_at_once_cannot_both_take_the_last_dollar(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 1).await;
    let mut top = db::scoped(&pool, ana).await.unwrap();
    ledger::top_up(&mut top, 250, 50_000).await.unwrap(); // $0.50 more
    top.commit().await.unwrap();

    let request = shot(None);
    let (one, two) = tokio::join!(start(&pool, ana, &request), start(&pool, ana, &request));
    assert!(one.is_ok() != two.is_ok(), "{one:?} {two:?}");
    assert_eq!(balance(&pool, ana).await, 1_500_000 - SHOT_PRICE);
}

#[sqlx::test]
async fn a_spoken_line_is_priced_from_its_quote(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 1).await;
    let settings = json!({ "speed": 1.0 });
    let line = Request::Line(Line {
        project: None,
        tool_call: None,
        job: None,
        model: "fast",
        voice: "matilda",
        text: "Olá, mundo.",
        settings: &settings,
        estimated_cents: 1,
    });
    let paid = start(&pool, ana, &line).await.unwrap();
    assert_eq!(paid.reservation.micros, 11_000, "one cent plus ten percent");
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();
    assert_eq!(balance(&pool, ana).await, 1_000_000 - 11_000);
}

#[sqlx::test]
async fn an_assistant_call_is_charged_from_its_token_counts(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let usage = Usage {
        input: 2_000,
        output: 600,
        cache_read: 50_000,
        ..Usage::default()
    };
    let call = AssistantCall {
        model: MODEL,
        usage,
        project: Some(3),
        prompt: "cut the intro",
    };
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let charged = ledger::charge_assistant(&mut tx, &call).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(charged, price(30_000));
    assert_eq!(charged, 33_000);
    assert_eq!(balance(&pool, ana).await, -33_000, "charged, not refused");
}
