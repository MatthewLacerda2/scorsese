//! A quote left unanswered: a message written instead is the answer, and a
//! Stop sets it aside — either way the quote is withdrawn (#1440).

use sqlx::postgres::PgPool;

use super::{never_told_a_token, quoted, quotes_held, turns};
use crate::{Script, answers, common, finished, send};

#[sqlx::test]
async fn a_message_written_instead_withdraws_the_quote_and_resumes_with_it(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, who, id, detail) = quoted(&pool, &script).await;
    let first = detail["turn"]["id"].as_i64().unwrap();
    script.replace(vec![answers("A shorter line, then.")]);
    let sent = send(address, &who, id, "make the line shorter first").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    assert_eq!(sent.json()["id"], first, "the paused turn resumes");
    assert_eq!(quotes_held(&pool).await, 0);

    let log = finished(address, &who, first).await;
    assert_eq!(log["turn"]["answer"], "A shorter line, then.");
    assert_eq!(log["turn"]["quote_answer"], "withdrawn");
    let told = script.messages(1).last().unwrap().clone();
    assert!(told.contains("did not answer the quote"), "{told}");
    assert!(told.contains("make the line shorter first"), "{told}");
    assert_eq!(turns(&pool).await, 1);
    never_told_a_token(&script);
}

#[sqlx::test]
async fn a_stop_sets_the_quote_aside_and_withdraws_it(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, who, id, detail) = quoted(&pool, &script).await;
    let first = detail["turn"]["id"].as_i64().unwrap();
    let path = format!("/api/chat/turns/{first}/stop");
    let stopped = common::request(address, "POST", &path, &[&who], None).await;
    assert_eq!(stopped.status, 202, "{}", stopped.body);
    let log = finished(address, &who, first).await;
    assert_eq!(log["turn"]["state"], "stopped", "{log}");
    assert_eq!(log["turn"]["quote_answer"], "withdrawn");
    assert_eq!(quotes_held(&pool).await, 0);

    script.replace(vec![answers("Leaving it.")]);
    let sent = send(address, &who, id, "never mind").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().unwrap()).await;
    let told = script.messages(1).join("\n");
    assert!(told.contains("instead of answering the quote"), "{told}");
}
