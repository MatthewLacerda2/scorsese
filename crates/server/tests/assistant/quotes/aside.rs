//! A quote nobody can answer any more leaves no box behind: a new
//! conversation and a turn that ends without pausing both withdraw it
//! (#1440).

use std::time::Duration;

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{answer, narrated, quoted, quotes_held};
use crate::{Script, answers, calls, common, finished, member, project, scripted};

#[sqlx::test]
async fn a_new_conversation_withdraws_the_paused_quote(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, who, id, detail) = quoted(&pool, &script).await;
    let first = detail["turn"]["id"].as_i64().unwrap();
    assert_eq!(quotes_held(&pool).await, 1);

    script.replace(vec![answers("Starting over.")]);
    let path = format!("/api/projects/{id}/chat");
    let body = json!({ "prompt": "start again", "fresh": true });
    let sent = common::request(address, "POST", &path, &[&who], Some(&body)).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    assert_ne!(sent.json()["id"], first, "a new conversation, a new turn");

    let old = finished(address, &who, first).await;
    assert_eq!(old["turn"]["state"], "stopped", "{old}");
    assert_eq!(old["turn"]["quote_answer"], "withdrawn");
    assert_eq!(old["turn"]["quote_replies"][0]["answer"], "withdrawn");
    assert_eq!(
        quotes_held(&pool).await,
        0,
        "withdrawn from the paid tool too"
    );
    let late = answer(address, &who, &detail, true).await;
    assert_eq!(late.status, 400, "nothing to answer: {}", late.body);
}

#[sqlx::test]
async fn a_stop_heard_while_quoting_ends_the_turn_and_withdraws_the_quote(pool: PgPool) {
    let script = Script::slow(vec![], Duration::from_millis(300));
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, narrated()).await;
    script.replace(vec![calls("generate", json!({ "project": id }))]);
    let sent = crate::send(address, &who, id, "narrate it").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().unwrap();
    // Asked for while the model is still replying with its quoting call.
    let path = format!("/api/chat/turns/{turn}/stop");
    let stopped = common::request(address, "POST", &path, &[&who], None).await;
    assert_eq!(stopped.status, 202, "{}", stopped.body);

    let log = finished(address, &who, turn).await;
    assert_eq!(log["turn"]["state"], "stopped", "{log}");
    assert!(log["turn"]["quote"].is_object(), "it did quote: {log}");
    assert_eq!(log["turn"]["quote_answer"], "withdrawn", "no dead box");
    assert_eq!(quotes_held(&pool).await, 0);
    let late = answer(address, &who, &log, true).await;
    assert_eq!(late.status, 400, "nothing to answer: {}", late.body);
}
