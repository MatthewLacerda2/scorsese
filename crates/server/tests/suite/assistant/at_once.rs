//! One person's turns at once: at most [`DEFAULT_TURNS_AT_ONCE`] running,
//! every project together, and none admitted on the money another's call in
//! flight may spend.

use std::time::Duration;

use scorsese_server::assistant::DEFAULT_TURNS_AT_ONCE;
use scorsese_server::credits::ledger;
use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, finished, member, project, send};

/// Wait until `script` has been sent `count` requests.
async fn sent(script: &Script, count: usize) {
    for _ in 0..200 {
        if script.requests().len() >= count {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the model was never sent {count} requests");
}

#[sqlx::test]
async fn a_person_runs_only_so_many_turns_at_once(pool: PgPool) {
    let replies = (0..=DEFAULT_TURNS_AT_ONCE)
        .map(|_| answers("Done."))
        .collect();
    let script = Script::slow(replies, Duration::from_millis(400));
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let mut running = Vec::new();
    for _ in 0..DEFAULT_TURNS_AT_ONCE {
        let id = project(&pool, ana, json!([])).await;
        let started = send(address, &who, id, "make an intro").await;
        assert_eq!(started.status, 202, "{}", started.body);
        running.push(started.json()["id"].as_i64().unwrap());
    }

    // Another project's conversation is no way round it.
    let another = project(&pool, ana, json!([])).await;
    let refused = send(address, &who, another, "and a trailer").await;
    assert_eq!(refused.status, 409, "{}", refused.body);
    assert!(
        refused.body.contains("at once"),
        "the limit is said: {}",
        refused.body
    );

    // Somebody else is not held up by ana.
    let (bia, bia_who) = member(&pool, "bia@example.com", 10).await;
    let hers = project(&pool, bia, json!([])).await;
    let started = send(address, &bia_who, hers, "make an intro").await;
    assert_eq!(started.status, 202, "{}", started.body);

    // Once one has finished, the next is taken.
    finished(address, &who, running[0]).await;
    let taken = send(address, &who, another, "and a trailer").await;
    assert_eq!(taken.status, 202, "{}", taken.body);
}

#[sqlx::test]
async fn a_call_in_flight_holds_what_it_may_cost(pool: PgPool) {
    let replies = vec![answers("Done."), answers("Done too.")];
    let script = Script::slow(replies, Duration::from_millis(600));
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 0).await;
    // Ten cents: enough to start a turn, not to pay for two calls at once.
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    ledger::top_up(&mut tx, 100_000).await.unwrap();
    tx.commit().await.unwrap();

    let first = project(&pool, ana, json!([])).await;
    let started = send(address, &who, first, "make an intro").await;
    assert_eq!(started.status, 202, "{}", started.body);
    sent(&script, 1).await;

    let second = project(&pool, ana, json!([])).await;
    let refused = send(address, &who, second, "and a trailer").await;
    assert_eq!(refused.status, 409, "{}", refused.body);
    assert!(
        refused.body.contains("being worked on"),
        "why is said: {}",
        refused.body
    );
    assert_eq!(script.requests().len(), 1, "nothing more was sent");

    // Once the first call is charged, what is left is ana's to spend again.
    let turn = started.json()["id"].as_i64().unwrap();
    let detail = finished(address, &who, turn).await;
    assert_eq!(detail["turn"]["state"], "answered", "{detail}");
    let taken = send(address, &who, second, "and a trailer").await;
    assert_eq!(taken.status, 202, "{}", taken.body);
}
