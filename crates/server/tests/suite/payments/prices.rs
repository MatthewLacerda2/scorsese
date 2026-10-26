//! `GET /api/prices` (#1538): what the public pricing page shows, read
//! without signing in, from the tables that charge.

use std::sync::Arc;

use scorsese_server::payments::Bonus;
use sqlx::postgres::PgPool;

use super::bonus::{EIGHTY, FORTY};
use super::common::request;
use super::{Fake, rates, serve, serve_with};

#[sqlx::test]
async fn a_visitor_reads_each_cards_dollars_at_the_days_rate(pool: PgPool) {
    let latest = rates(Some("4.99"));
    latest.refresh().expect("the stand-in answers");
    let fake = Some(Arc::new(Fake::default()));
    let address = serve_with(&pool, fake, &latest, Bonus::default()).await;

    let read = request(address, "GET", "/api/prices", &[], None).await;
    assert_eq!(read.status, 200, "{}", read.body);
    let prices = read.json();
    let ids: Vec<_> = prices["examples"]
        .as_array()
        .expect("examples are a list")
        .iter()
        .map(|example| example["id"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(ids, ["image", "narration", "music", "video"]);
    let cards = &prices["recharge"]["cards"];
    let summary: Vec<_> = (0..3)
        .map(|i| {
            let card = &cards[i];
            (
                card["brl_centavos"].as_i64(),
                card["bonus_percent"].as_i64(),
                card["best"].as_bool(),
                card["quote"]["brl_per_usd"].as_str().map(str::to_owned),
            )
        })
        .collect();
    let rate = Some("4.99".to_owned());
    assert_eq!(
        summary,
        [
            (Some(2_000), Some(0), Some(false), rate.clone()),
            (Some(4_000), Some(5), Some(false), rate.clone()),
            (Some(8_000), Some(10), Some(true), rate),
        ]
    );
    assert_eq!(cards[1]["quote"]["credit_micros"], FORTY.0);
    assert_eq!(cards[1]["quote"]["bonus_micros"], FORTY.1);
    assert_eq!(cards[2]["quote"]["bonus_micros"], EIGHTY.1);
    assert!(cards.get(3).is_none(), "{cards}");
}

#[sqlx::test]
async fn with_no_rate_kept_the_cards_come_without_dollars(pool: PgPool) {
    let address = serve(&pool, Some(Arc::new(Fake::default())), Some("4.99")).await;
    let prices = request(address, "GET", "/api/prices", &[], None)
        .await
        .json();
    let cards = prices["recharge"]["cards"]
        .as_array()
        .expect("cards are a list");
    assert_eq!(cards.len(), 1, "no bonus tiers: only the least purchase");
    assert!(cards[0]["quote"].is_null(), "{prices}");
}

#[sqlx::test]
async fn a_server_that_sells_nothing_still_states_its_prices(pool: PgPool) {
    let address = serve(&pool, None, None).await;
    let read = request(address, "GET", "/api/prices", &[], None).await;
    assert_eq!(read.status, 200, "{}", read.body);
    let prices = read.json();
    assert!(prices["recharge"].is_null());
    assert_eq!(prices["free"]["daily_micros"], 0);
    assert!(!prices["examples"].as_array().expect("a list").is_empty());
}
