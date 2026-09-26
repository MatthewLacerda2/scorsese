//! `spending_history` describes itself like every other tool, and answers in
//! words an assistant can quote.

use scorsese_server::credits::generations::Answer;
use scorsese_server::credits::history;
use scorsese_server::credits::tool::{DESCRIPTION, NAME, answer, filter, schema};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{account, finish, fund, shot, start};

/// `docs/mcp.md`'s rule: the tool and every argument carry a description.
#[test]
fn the_tool_and_every_argument_are_described() {
    assert_eq!(NAME, "spending_history");
    let first = DESCRIPTION.split(". ").next().unwrap();
    assert!(first.len() > 40, "the first sentence stands alone: {first}");
    let schema = schema();
    let properties = schema["properties"].as_object().unwrap();
    assert!(!properties.is_empty());
    for (name, property) in properties {
        let described = property["description"].as_str().unwrap_or("");
        assert!(described.len() > 10, "{name} is not described");
    }
}

#[test]
fn arguments_become_a_filter() {
    let asked = filter(&json!({ "kind": "veo_shot", "since": "2026-09-21" })).unwrap();
    assert_eq!(asked.kind.as_deref(), Some("veo_shot"));
    assert_eq!(asked.since.as_deref(), Some("2026-09-21"));
    assert!(filter(&json!({ "project": "seven" })).is_err());
}

#[sqlx::test]
async fn the_answer_says_what_was_charged_and_what_was_free(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let paid = start(&pool, ana, &shot(Some(9))).await.unwrap();
    finish(&pool, ana, paid, &Answer::Failed("quota".into()))
        .await
        .unwrap();
    let seen = history::read(&pool, ana, &Default::default())
        .await
        .unwrap();
    let text = answer(&seen);
    assert!(text.starts_with("Balance: $10.00.\n"), "{text}");
    assert!(text.contains("dollars only"), "{text}");
    assert!(
        text.contains("project 9, since deleted: free: the provider failed"),
        "{text}"
    );
    assert!(
        text.contains("Top-up: $10.00; balance after $10.00"),
        "{text}"
    );
}
