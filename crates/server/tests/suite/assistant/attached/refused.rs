//! A message the assistant refuses changes nothing: its attachments are
//! brought into the project only once the turn may start, so an empty prompt,
//! a missing key, an empty balance, a running turn or the edit limit imports
//! nothing.

use std::time::Duration;

use scorsese_server::assistant::Assistant;
use scorsese_server::http::limits::Limits;
use scorsese_server::library::Kind;
use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::{Script, answers, common, finished, member, project, scripted, serve};
use super::{kept, send_with};

/// Whether `project` of `user`'s still has no assets.
async fn untouched(pool: &PgPool, user: scorsese_server::db::UserId, project: i64) -> bool {
    let document = projects::open(pool, user, project)
        .await
        .expect("opens")
        .document;
    document.assets.is_empty()
}

#[sqlx::test]
async fn an_empty_prompt_imports_nothing(pool: PgPool) {
    let script = Script::new(vec![answers("never")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let logo = kept(&state, ana, "logo.png", Kind::Image).await;
    let refused = send_with(address, &who, id, "  ", json!([logo])).await;
    assert_eq!(refused.status, 400, "{}", refused.body);
    assert!(untouched(&pool, ana, id).await, "nothing was imported");
}

#[sqlx::test]
async fn an_empty_balance_imports_nothing(pool: PgPool) {
    let script = Script::new(vec![answers("never")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 0).await;
    let id = project(&pool, ana, json!([])).await;
    let logo = kept(&state, ana, "logo.png", Kind::Image).await;
    let refused = send_with(address, &who, id, "use it", json!([logo])).await;
    assert_eq!(refused.status, 402, "{}", refused.body);
    assert!(untouched(&pool, ana, id).await, "nothing was imported");
}

#[sqlx::test]
async fn without_a_key_nothing_is_imported(pool: PgPool) {
    let (address, state) = serve(&pool, Assistant::default().unconfigured()).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let logo = kept(&state, ana, "logo.png", Kind::Image).await;
    let refused = send_with(address, &who, id, "use it", json!([logo])).await;
    assert_eq!(refused.status, 503, "{}", refused.body);
    assert!(untouched(&pool, ana, id).await, "nothing was imported");
}

#[sqlx::test]
async fn more_files_than_a_message_carries_are_refused(pool: PgPool) {
    let script = Script::new(vec![answers("never")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let many: Vec<i64> = (1..=21).collect();
    let refused = send_with(address, &who, id, "use them", json!(many)).await;
    assert_eq!(refused.status, 400, "{}", refused.body);
    assert!(refused.body.contains("at most 20"), "{}", refused.body);
    assert!(script.requests().is_empty(), "no turn started");
}

#[sqlx::test]
async fn files_sent_while_a_turn_runs_are_not_imported(pool: PgPool) {
    let script = Script::slow(vec![answers("Looking.")], Duration::from_millis(300));
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let logo = kept(&state, ana, "logo.png", Kind::Image).await;
    let running = send_with(address, &who, id, "look around", json!([])).await;
    assert_eq!(running.status, 202, "{}", running.body);
    let refused = send_with(address, &who, id, "use it", json!([logo])).await;
    assert_eq!(refused.status, 409, "{}", refused.body);
    assert!(untouched(&pool, ana, id).await, "nothing was imported");
    let turn = running.json()["id"].as_i64().expect("a turn id");
    finished(address, &who, turn).await;
}

#[sqlx::test]
async fn past_the_edit_limit_nothing_is_imported(pool: PgPool) {
    let script = Script::new(vec![answers("never")]);
    let (_, mut state) = scripted(&pool, &script).await;
    // The server under test was made before this limit; a second router over
    // the same state, with no edits left this minute, is the one exercised.
    state.limits = Limits::new(0);
    let (listener, address) = common::listener().await;
    let members = state.pool.clone();
    let router = scorsese_server::http::router(state.clone());
    let server = tokio::spawn(scorsese_server::http::serve(
        listener,
        router,
        std::future::pending(),
    ));
    common::release_at_teardown(&pool, &members, [server.abort_handle()]);
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let logo = kept(&state, ana, "logo.png", Kind::Image).await;
    let refused = send_with(address, &who, id, "use it", json!([logo])).await;
    assert_eq!(refused.status, 429, "{}", refused.body);
    assert!(untouched(&pool, ana, id).await, "nothing was imported");
    assert!(script.requests().is_empty(), "no turn started");
}
