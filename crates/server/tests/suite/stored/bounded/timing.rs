//! At most three at once, and none past its deadline.

use std::sync::Arc;
use std::time::{Duration, Instant};

use scorsese_server::tools::Client;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Gate, arrived, held};

#[sqlx::test]
async fn a_fourth_call_at_once_waits_for_one_of_the_three(pool: PgPool) {
    let gate = Arc::new(Gate::default());
    let (state, ana, id, _) = held(&pool, "bounded-at-once", &gate).await;
    // A query each: an answer the stock cache holds never reaches the library.
    let calls: Vec<_> = (0..4)
        .map(|n| {
            let state = state.clone();
            let search = json!({ "project": id, "query": format!("sea {n}") });
            tokio::spawn(async move {
                let client = Client::External { token: None };
                state.tools.call(ana, client, "stock_search", &search).await
            })
        })
        .collect();

    arrived(&gate, 3).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(gate.entered(), 3, "the fourth waits for a permit");

    gate.open();
    for call in calls {
        let answered = call.await.expect("the call ends");
        assert!(answered.is_ok(), "{:?}", answered.err());
    }
    assert_eq!(gate.entered(), 4, "and runs once one is free");
}

#[sqlx::test]
async fn a_call_past_its_deadline_ends_saying_so(pool: PgPool) {
    let gate = Arc::new(Gate::default());
    let (mut state, ana, id, _) = held(&pool, "bounded-deadline", &gate).await;
    state.tools = state.tools.clone().bounded(3, Duration::from_millis(300));
    let search = json!({ "project": id, "query": "sea" });
    let client = Client::External { token: None };

    let asked = Instant::now();
    let answered = state.tools.call(ana, client, "stock_search", &search).await;
    let said = answered.err().expect("refused past its deadline");
    assert!(said.contains("stopped"), "{said}");
    assert!(
        asked.elapsed() < Duration::from_secs(10),
        "answered at the deadline"
    );
    gate.open();
}
