//! A yes or a no resumes the turn the quote paused: no new turn, no line
//! nobody typed, and no "still working" refusal (#1440).

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{answer, never_told_a_token, quoted, quotes_held, turns};
use crate::assistant::{Script, answers, finished};

#[sqlx::test]
async fn a_yes_spends_as_the_user_and_resumes_the_same_turn_with_its_reply(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, who, _, detail) = quoted(&pool, &script).await;
    let first = detail["turn"]["id"].as_i64().unwrap();
    script.replace(vec![answers("The line is being made.")]);
    let answered = answer(address, &who, &detail, true).await;
    assert_eq!(
        answered.status, 200,
        "a yes is never busy: {}",
        answered.body
    );
    let answered = answered.json();
    assert_eq!(answered["refused"], false, "{answered}");
    let spent = answered["spent"].as_str().unwrap();
    assert!(spent.contains("queued as job"), "{answered}");
    assert_eq!(answered["turn"]["id"], first, "the same turn carries on");

    let log = finished(address, &who, first).await;
    assert_eq!(log["turn"]["answer"], "The line is being made.");
    assert_eq!(log["turn"]["quote_answer"], "confirmed");
    let yes = log["tools"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(
        (yes["client"].clone(), yes["tool"].clone()),
        (json!("user"), json!("generate"))
    );
    assert_eq!(turns(&pool).await, 1, "no turn was started");
    let told = script.messages(1).last().unwrap().clone();
    assert!(
        told.contains("tool_result"),
        "the call's own result: {told}"
    );
    assert!(told.contains("The person confirmed the quote"), "{told}");
    assert!(told.contains("queued as job"), "{told}");
    assert!(!told.contains("waits for their answer"), "{told}");
    never_told_a_token(&script);
    let again = answer(address, &who, &detail, true).await;
    assert_eq!(again.status, 400, "answered once: {}", again.body);
}

#[sqlx::test]
async fn a_no_withdraws_the_quote_and_resumes_the_turn_with_it(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, who, _, detail) = quoted(&pool, &script).await;
    let first = detail["turn"]["id"].as_i64().unwrap();
    script.replace(vec![answers("Understood; nothing was made.")]);
    let declined = answer(address, &who, &detail, false).await;
    assert_eq!(declined.status, 200, "{}", declined.body);
    assert_eq!(declined.json()["turn"]["id"], first);
    assert_eq!(quotes_held(&pool).await, 0);

    let log = finished(address, &who, first).await;
    assert_eq!(log["turn"]["answer"], "Understood; nothing was made.");
    assert_eq!(log["turn"]["quote_answer"], "declined");
    let told = script.messages(1).last().unwrap().clone();
    assert!(told.contains("declined the quote"), "{told}");
    assert_eq!(turns(&pool).await, 1);
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs, 0);
    never_told_a_token(&script);
}
