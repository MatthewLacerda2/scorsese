//! A summary that does not come back — stopped on a tool, refused — leaves
//! the turn running on the whole conversation, with nothing shown to the
//! person; a dropped connection is retried; and a summarising call never
//! counts towards the size the next turn compares.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::script::CHARGED;
use super::super::{answers, calls, exchange, member, project, scripted};
use super::{Script, sized, summarising};

/// A script whose first call read past the threshold, and the replies after.
fn long(then: Vec<scorsese_providers::chat::Reply>) -> std::sync::Arc<Script> {
    let mut replies = vec![sized(answers("Here is the script."), 150_001)];
    replies.extend(then);
    Script::new(replies)
}

#[sqlx::test]
async fn a_summary_stopped_on_a_tool_is_asked_once_more_then_given_up(pool: PgPool) {
    let script = long(vec![
        calls("still", json!({ "at": 1.0 })),
        calls("still", json!({ "at": 2.0 })),
        answers("Narration added."),
    ]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    exchange(address, &who, id, "write the script").await;

    let detail = exchange(address, &who, id, "add the narration").await;
    assert_eq!(detail["turn"]["state"], "answered", "{detail}");
    assert_eq!(detail["turn"]["answer"], "Narration added.");
    assert_eq!(
        detail["turn"]["charged_micros"],
        3 * CHARGED,
        "both asks paid"
    );
    assert!(summarising(&script, 1) && summarising(&script, 2));
    // Uncompacted: the turn re-reads the whole conversation, prefix intact.
    let (earlier, now) = (script.messages(0), script.messages(3));
    assert_eq!(now[..earlier.len()], earlier[..], "{now:?}");
}

#[sqlx::test]
async fn a_summary_stopped_on_a_tool_once_is_taken_the_second_time(pool: PgPool) {
    let script = long(vec![
        calls("still", json!({ "at": 1.0 })),
        answers("SUMMARY: the script is written."),
        answers("Narration added."),
    ]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    exchange(address, &who, id, "write the script").await;
    exchange(address, &who, id, "add the narration").await;

    let opened = script.messages(3).concat();
    assert!(opened.contains("SUMMARY: the script"), "{opened}");
    assert!(!opened.contains("Here is the script."), "{opened}");
}

#[sqlx::test]
async fn a_refused_summary_leaves_the_turn_uncompacted_and_unharmed(pool: PgPool) {
    let script = long(vec![answers("Narration added.")]);
    script.fail(1, false);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    exchange(address, &who, id, "write the script").await;

    let detail = exchange(address, &who, id, "add the narration").await;
    assert_eq!(detail["turn"]["state"], "answered", "{detail}");
    assert_eq!(detail["turn"]["answer"], "Narration added.");
    assert_eq!(detail["turn"]["calls"], 1, "a refusal is not a call");
    let (earlier, now) = (script.messages(0), script.messages(2));
    assert_eq!(now[..earlier.len()], earlier[..], "{now:?}");
}

#[sqlx::test]
async fn a_dropped_summary_is_sent_again(pool: PgPool) {
    let script = long(vec![
        answers("SUMMARY: the script is written."),
        answers("Narration added."),
    ]);
    script.fail(1, true);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    exchange(address, &who, id, "write the script").await;
    exchange(address, &who, id, "add the narration").await;

    assert!(
        summarising(&script, 1) && summarising(&script, 2),
        "retried"
    );
    let opened = script.messages(3).concat();
    assert!(opened.contains("SUMMARY: the script"), "{opened}");
}

#[sqlx::test]
async fn a_summary_never_counts_towards_the_next_turns_size(pool: PgPool) {
    // The summary read the whole old conversation; the compacted turn's own
    // call failed, so nothing after the summary says how big it is now.
    let script = long(vec![
        sized(answers("SUMMARY: the script is written."), 150_500),
        answers("Louder now."),
    ]);
    script.fail(2, false);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    exchange(address, &who, id, "write the script").await;
    let failed = exchange(address, &who, id, "add the narration").await;
    assert_eq!(failed["turn"]["state"], "failed", "{failed}");

    let detail = exchange(address, &who, id, "louder").await;
    assert_eq!(detail["turn"]["answer"], "Louder now.", "{detail}");
    assert_eq!(script.requests().len(), 4, "no second summary");
    assert!(!summarising(&script, 3));
    let (before, now) = (script.messages(2), script.messages(3));
    assert_eq!(now[..before.len()], before[..], "from the summary on");
}
