//! A turn the vendor stopped for a reason of its own is a warning naming the
//! turn and its owner, kept under a filter keeping warnings alone (#1520).

use scorsese_providers::chat::Stop;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::common::logged;
use super::{Script, answers, exchange, member, project, scripted};

#[sqlx::test]
async fn a_turn_the_vendor_stopped_is_a_warning_naming_it(pool: PgPool) {
    let (_logging, kept) = logged::keep_at(Some("warn"));
    let mut stopped = answers("");
    stopped.stop = Stop::Other("pause_turn_forever".into());
    let script = Script::new(vec![stopped]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;

    let detail = exchange(address, &who, id, "make a title").await;
    assert_eq!(detail["turn"]["state"], "failed", "{detail}");

    let log = kept.text();
    let named = format!("turn{{id={} user={}}}", detail["turn"]["id"], ana.get());
    assert!(
        log.lines().any(|line| line.contains(&named)
            && line.contains(" WARN ")
            && line.contains("pause_turn_forever")),
        "{named} in:\n{log}"
    );
}
