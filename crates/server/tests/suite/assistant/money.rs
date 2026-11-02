//! Money: a turn is refused on an empty balance, capped per turn, charged per
//! call against the turn — and without a key the rest of the server is fine.

use scorsese_server::assistant::Assistant;
use scorsese_server::credits::allowance::{self, Allowance};
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
    // The project is on Opus 5.5, whose cap is twice the Sonnet baseline: one
    // call reaches it.
    let capped = Assistant::new(CHARGED / 2).answered_by(script.clone());
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

/// A new account with no credit runs a turn on the free allowance, which
/// pays all of it (#1530).
#[sqlx::test]
async fn a_new_account_runs_a_turn_on_the_free_allowance(pool: PgPool) {
    let script = Script::new(vec![answers("Here is an intro.")]);
    let (address, _) = super::scripted(&pool, &script).await;
    allowance::set(&pool, Allowance::DEFAULT).await.unwrap();
    let (ana, who) = member(&pool, "ana@example.com", 0).await;
    let id = project(&pool, ana, json!([])).await;
    let detail = exchange(address, &who, id, "make an intro").await;
    assert_eq!(detail["turn"]["state"], "answered", "{detail}");

    let seen = history::read(&pool, ana, &Filter::default()).await.unwrap();
    assert_eq!(seen.balance_micros, 0, "nothing paid, nothing owed");
    assert_eq!(seen.rows[0].amount_micros, 0);
    assert_eq!(seen.rows[0].free_micros, CHARGED);
}

/// A call is refused before it is sent when what it would cost — its input,
/// and a reply — is more than is left (#1653): five cents is not a call on
/// Opus 5.5, whose system prompt and tools alone cost more than that.
#[sqlx::test]
async fn a_call_that_costs_more_than_is_left_is_never_sent(pool: PgPool) {
    let script = Script::new(vec![answers("never")]);
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 0).await;
    let mut tx = scorsese_server::db::scoped(&pool, ana).await.unwrap();
    scorsese_server::credits::ledger::top_up(&mut tx, 50_000)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let id = project(&pool, ana, json!([])).await;
    let refused = send(address, &who, id, "make an intro").await;
    assert_eq!(refused.status, 402, "{}", refused.body);
    assert!(refused.body.contains("add credit"), "{}", refused.body);
    assert!(script.requests().is_empty(), "nothing was sent");
}

/// A turn that starts, since what is left pays for a short reply, ends
/// before its first call when it does not pay for that call's input too
/// (#1653): thirty cents is not Opus 5.5's system prompt and tools written to
/// the cache. The conversation is kept.
#[sqlx::test]
async fn a_turn_ends_before_a_call_its_balance_cannot_pay_for(pool: PgPool) {
    let script = Script::new(vec![answers("never")]);
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 0).await;
    let mut tx = scorsese_server::db::scoped(&pool, ana).await.unwrap();
    scorsese_server::credits::ledger::top_up(&mut tx, 300_000)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let id = project(&pool, ana, json!([])).await;
    let detail = exchange(address, &who, id, "make an intro").await;
    assert_eq!(detail["turn"]["state"], "capped", "{detail}");
    let answer = detail["turn"]["answer"].as_str().unwrap();
    assert!(answer.contains("Add credit"), "{answer}");
    assert!(script.requests().is_empty(), "nothing was sent");
    let chat = format!("/api/projects/{id}/chat");
    let kept = common::request(address, "GET", &chat, &[&who], None).await;
    assert_eq!(kept.json()["turns"][0]["prompt"], "make an intro");
}

/// A reply cut short because what was left paid for no more of it ends the
/// turn with the out-of-credit sentence, not as a reply past its length
/// (#1653): a dollar pays for far less than a whole reply on Opus 5.5.
#[sqlx::test]
async fn a_reply_cut_short_by_the_balance_says_to_add_credit(pool: PgPool) {
    let cut = super::script::reply(
        scorsese_providers::chat::Part::Text {
            text: "Half of".into(),
        },
        scorsese_providers::chat::Stop::MaxTokens,
    );
    let script = Script::new(vec![cut]);
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 1).await;
    let id = project(&pool, ana, json!([])).await;
    let detail = exchange(address, &who, id, "make an intro").await;
    assert_eq!(detail["turn"]["state"], "capped", "{detail}");
    let answer = detail["turn"]["answer"].as_str().unwrap();
    assert!(answer.contains("Add credit"), "{answer}");
    assert_eq!(script.requests().len(), 1);
}
