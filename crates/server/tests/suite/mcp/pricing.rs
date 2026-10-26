//! A quote prices each brief by its own kind (#1247): a shot at 1.5 times
//! its cost, a line at 1.8 — and says exactly what the yes reserves.

use scorsese_server::credits::{dollars, ledger};
use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::{fund, token};
use super::{call, common, member};

#[sqlx::test]
async fn a_shot_and_a_line_in_one_quote_are_each_priced_by_their_kind(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let both = json!({ "assets": [
        { "id": "hero", "kind": "generated_video", "state": "sketch",
          "prompt": "a lighthouse at dawn" },
        { "id": "vo", "kind": "generated_audio", "state": "sketch",
          "prompt": "Every city has a night editor.", "speech": { "voice_id": "voice-one" } }
    ] });
    let id = super::stored(&pool, ana, both).await;
    fund(&pool, ana, 10).await;

    let (quote, refused) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(!refused, "{quote}");
    let confirm = json!({ "project": id, "confirm": token(&quote) });
    let (said, refused) = call(address, &who, "generate", confirm).await;
    assert!(!refused, "{said}");

    let shot: i64 = sqlx::query_scalar("SELECT estimated_cost_micros FROM veo_generations")
        .fetch_one(&pool)
        .await
        .unwrap();
    let line: i64 = sqlx::query_scalar("SELECT estimated_cost_micros FROM speech_generations")
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let reserved = 10_000_000 - ledger::balance(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(reserved, shot * 3 / 2 + line * 9 / 5);
    let charged = format!("{} from your credits", dollars(reserved));
    assert!(quote.contains(&charged), "{charged}: {quote}");
}
