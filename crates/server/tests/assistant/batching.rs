//! The quote box's second yes (#947): stills worth a dollar are offered as a
//! half-price batch beside the price for now, and the person — never the
//! model — picks which one spends.

use std::net::SocketAddr;

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, exchange, member, project, scripted};

/// Twenty sketched stills: $1.32 drawn now, with the markup.
fn stills() -> Value {
    let assets: Vec<Value> = (0..20)
        .map(|n| {
            json!({ "id": format!("still{n}"), "kind": "generated_image", "state": "sketch",
                    "prompt": format!("A harbour at dusk, take {n}.") })
        })
        .collect();
    Value::Array(assets)
}

/// A turn whose model asks `generate` for a quote; the server's address, the
/// member, and the turn's detail.
async fn quoted(pool: &PgPool) -> (SocketAddr, String, Value) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(pool, &script).await;
    let (ana, who) = member(pool, "ana@example.com", 10).await;
    let id = project(pool, ana, stills()).await;
    script.replace(vec![
        calls("generate", json!({ "project": id })),
        answers("Twenty stills; the box has both prices."),
        answers("They are on their way."),
    ]);
    let detail = exchange(address, &who, id, "draw them").await;
    (address, who, detail)
}

/// `body`, as the answer to `turn`'s quote.
async fn answer(address: SocketAddr, who: &str, turn: &Value, body: Value) -> Value {
    let path = format!("/api/chat/turns/{}/quote", turn["turn"]["id"]);
    let answered = common::request(address, "POST", &path, &[who], Some(&body)).await;
    assert_eq!(answered.status, 200, "{}", answered.body);
    answered.json()
}

/// How many of each kind of job there are, and how many quotes are held.
async fn tally(pool: &PgPool) -> (i64, i64, i64) {
    let count = |sql: &'static str| async move {
        sqlx::query_scalar::<_, i64>(sql)
            .fetch_one(pool)
            .await
            .unwrap()
    };
    (
        count("SELECT count(*) FROM jobs WHERE kind = 'still_image'").await,
        count("SELECT count(*) FROM jobs WHERE kind = 'batch_still'").await,
        count("SELECT count(*) FROM quotes").await,
    )
}

#[sqlx::test]
async fn the_box_offers_the_batch_and_its_yes_orders_it(pool: PgPool) {
    let (address, who, detail) = quoted(&pool).await;
    let quote = &detail["turn"]["quote"];
    assert_eq!(quote["micros"], 1_320_000, "{quote}");
    assert_eq!(quote["batch_micros"], 660_000, "{quote}");
    assert!(!quote.to_string().contains("quote-"), "no token: {quote}");

    let answered = answer(
        address,
        &who,
        &detail,
        json!({ "confirm": true, "batch": true }),
    )
    .await;
    assert_eq!(answered["refused"], false, "{answered}");
    assert!(
        answered["spent"]
            .as_str()
            .unwrap()
            .contains("half-price batch")
    );
    assert_eq!(
        tally(&pool).await,
        (0, 20, 0),
        "the batch ordered, both tokens gone"
    );
}

#[sqlx::test]
async fn a_plain_yes_draws_now_and_forgets_the_offer(pool: PgPool) {
    let (address, who, detail) = quoted(&pool).await;
    let answered = answer(address, &who, &detail, json!({ "confirm": true })).await;
    assert_eq!(answered["refused"], false, "{answered}");
    assert_eq!(tally(&pool).await, (20, 0, 0));
}

#[sqlx::test]
async fn a_no_cannot_pick_the_batch(pool: PgPool) {
    let (address, who, detail) = quoted(&pool).await;
    let path = format!("/api/chat/turns/{}/quote", detail["turn"]["id"]);
    let body = json!({ "confirm": false, "batch": true });
    let refused = common::request(address, "POST", &path, &[&who], Some(&body)).await;
    assert_eq!(refused.status, 400, "{}", refused.body);
}
