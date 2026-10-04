//! What a turn leaves for analysis (#707): a typed `model_calls` row per call
//! to the model, and each tool reply's estimated size in tokens.

use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::script::USAGE;
use super::{Script, answers, calls, exchange, member, project};

/// One `model_calls` row: position, model, vendor, input, output, thinking,
/// latency, stop reason, cost, and whether its credit entry is the turn's.
type Row = (
    i32,
    String,
    String,
    i64,
    i64,
    Option<i64>,
    i64,
    String,
    i64,
    bool,
);

#[sqlx::test]
async fn each_call_is_a_typed_row_and_each_tool_reply_is_sized(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let mut done = answers("Done.");
    done.thinking = Some(7);
    script.replace(vec![calls("icons", json!({})), done]);
    let detail = exchange(address, &who, id, "look around").await;
    let turn = detail["turn"]["id"].as_i64().unwrap();

    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT m.position, m.model, m.vendor, m.input_tokens, m.output_tokens,
                m.thinking_tokens, m.latency_ms, m.stop_reason, m.cost_micros,
                c.chat_turn_id = m.turn_id
         FROM model_calls m JOIN credit_entries c ON c.id = m.credit_entry_id
         WHERE m.turn_id = $1 ORDER BY m.position",
    )
    .bind(turn)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    let (input, output) = (USAGE.input as i64, USAGE.output as i64);
    let row = |position, thinking, stop: &str| {
        let model = ("claude-opus-5-5".to_owned(), "anthropic".to_owned());
        (
            position,
            model.0,
            model.1,
            input,
            output,
            thinking,
            stop.to_owned(),
        )
    };
    let seen: Vec<_> = rows
        .iter()
        .map(|r| (r.0, r.1.clone(), r.2.clone(), r.3, r.4, r.5, r.7.clone()))
        .collect();
    assert_eq!(
        seen,
        vec![row(1, None, "tool_use"), row(2, Some(7), "end_turn")]
    );
    assert!(
        rows.iter().all(|r| r.6 >= 0 && r.8 == 14_000 && r.9),
        "{rows:?}"
    );

    let (reply, estimate): (String, Option<i64>) =
        sqlx::query_as("SELECT reply, reply_tokens_estimate FROM tool_calls WHERE turn_id = $1")
            .bind(turn)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(estimate, Some(reply.len().div_ceil(4) as i64));
    assert!(estimate.unwrap() > 0);
}
