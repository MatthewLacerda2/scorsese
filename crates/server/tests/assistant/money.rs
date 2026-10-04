//! Money: a turn is refused on an empty balance, capped per turn, charged per
//! call against the turn — and without a key the rest of the server is fine.

use scorsese_server::assistant::Assistant;
use scorsese_server::credits::history::{self, Filter};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::script::CHARGED;
use super::{Script, answers, calls, common, exchange, member, project, send, serve};

#[sqlx::test]
async fn an_empty_balance_is_refused_before_anything_is_sent(pool: PgPool) {
    let script = Script::new(vec![answers("never")]);
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 0).await;
    let id = project(&pool, ana, json!([])).await;
    let refused = send(address, &who, id, "make an intro").await;
    assert_eq!(refused.status, 402, "{}", refused.body);
    assert!(refused.body.contains("add credit"), "{}", refused.body);
    assert!(script.requests().is_empty());
}

#[sqlx::test]
async fn without_a_key_the_assistant_says_so_and_nothing_else_breaks(pool: PgPool) {
    let (address, _) = serve(&pool, Assistant::default().unconfigured()).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let refused = send(address, &who, id, "make an intro").await;
    assert_eq!(refused.status, 503, "{}", refused.body);
    assert!(refused.body.contains("not configured"), "{}", refused.body);
    let health = common::request(address, "GET", "/api/health", &[], None).await;
    assert_eq!(health.status, 200);
    let chat = format!("/api/projects/{id}/chat");
    let empty = common::request(address, "GET", &chat, &[&who], None).await;
    assert_eq!(empty.status, 200, "{}", empty.body);
    assert_eq!(empty.json()["turns"], json!([]));
}

#[sqlx::test]
async fn the_per_turn_cap_ends_a_turn_between_calls(pool: PgPool) {
    let script = Script::new(vec![]);
    let capped = Assistant::new(CHARGED).answered_by(script.clone());
    let (address, _) = serve(&pool, capped).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    script.replace(vec![calls("icons", json!({})), answers("never reached")]);

    let detail = exchange(address, &who, id, "look around").await;
    assert_eq!(detail["turn"]["state"], "capped", "{detail}");
    assert!(
        detail["turn"]["answer"]
            .as_str()
            .unwrap()
            .contains("the most one message may")
    );
    assert_eq!(script.requests().len(), 1);
    assert_eq!(detail["turn"]["charged_micros"], CHARGED);
}

#[sqlx::test]
async fn a_turns_calls_are_one_row_of_the_spending_history(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    script.replace(vec![calls("icons", json!({})), answers("Done.")]);
    let detail = exchange(address, &who, id, "look around").await;

    let seen = history::read(&pool, ana, &Filter::default()).await.unwrap();
    let turn = &seen.rows[0];
    assert_eq!(turn.kind, "assistant");
    assert_eq!(turn.memo, "Assistant: look around");
    assert_eq!(turn.amount_micros, -2 * CHARGED);
    assert_eq!(turn.project_id, Some(id));
    assert_eq!(turn.detail["turn"], detail["turn"]["id"]);
    assert_eq!(turn.detail["calls"], 2);
    assert_eq!(seen.rows.len(), 2, "the turn, and the top-up");
    assert_eq!(seen.balance_micros, 10_000_000 - 2 * CHARGED);
}
