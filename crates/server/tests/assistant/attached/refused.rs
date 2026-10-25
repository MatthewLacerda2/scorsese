//! A message the assistant refuses changes nothing: its attachments are
//! brought into the project only once the turn may start, so an empty prompt,
//! a missing key or an empty balance imports nothing.

use scorsese_server::assistant::Assistant;
use scorsese_server::library::Kind;
use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::{Script, answers, member, project, scripted, serve};
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
