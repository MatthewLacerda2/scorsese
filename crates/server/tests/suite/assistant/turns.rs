//! A turn calls tools until the model answers, logs every call, and the next
//! turn replays the conversation exactly as it was sent.

use std::time::Duration;

use serde_json::json;
use sqlx::postgres::PgPool;

use super::script::CHARGED;
use super::{Script, answers, calls, common, exchange, finished, member, project, scripted, send};

#[sqlx::test]
async fn a_turn_runs_the_tools_the_model_asks_for_and_ends_with_its_answer(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    script.replace(vec![
        calls("project_read", json!({ "project": id })),
        answers("The project is empty; nothing to change."),
    ]);

    let detail = exchange(address, &who, id, "what is in my video?").await;
    let turn = &detail["turn"];
    assert_eq!(turn["state"], "answered", "{detail}");
    assert_eq!(turn["answer"], "The project is empty; nothing to change.");
    assert_eq!(turn["calls"], 2);
    assert_eq!(turn["charged_micros"], 2 * CHARGED);
    assert_eq!(turn["output_tokens"], 1_000);
    let tools = detail["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1, "{detail}");
    assert_eq!(tools[0]["tool"], "project_read");
    assert_eq!(tools[0]["client"], "assistant");
    assert_eq!(tools[0]["position"], 1);
    assert_eq!(tools[0]["outcome"], "answered", "{detail}");

    // The first request: the prompt, and the server's word on which project.
    let first = script.parsed(0);
    assert_eq!(first[0]["role"], "user");
    assert_eq!(first[0]["content"][0]["text"], "what is in my video?");
    assert_eq!(first[1]["role"], "system");
    assert!(
        first[1]["content"]
            .as_str()
            .unwrap()
            .contains(&format!("project {id}"))
    );
    // The second: the model's reply unchanged, then the tool's result.
    let second = script.parsed(1);
    let call = &second[2]["content"][1];
    assert_eq!(call["type"], "tool_use");
    let result = &second[3]["content"][0];
    assert_eq!(result["type"], "tool_result");
    assert_eq!(result["tool_use_id"], call["id"]);
    assert!(result.get("is_error").is_none(), "{result}");
}

#[sqlx::test]
async fn the_next_turn_resends_the_conversation_byte_for_byte(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    script.replace(vec![
        calls("icons", json!({})),
        answers("Here are the icons."),
        answers("You are welcome."),
    ]);
    exchange(address, &who, id, "which icons are there?").await;
    let detail = exchange(address, &who, id, "thanks").await;
    assert_eq!(detail["turn"]["answer"], "You are welcome.");

    let (earlier, now) = (script.messages(1), script.messages(2));
    assert_eq!(
        &now[..earlier.len()],
        &earlier[..],
        "history is appended, never rewritten"
    );
    // What followed: the model's answer, then the new prompt — no project
    // note this time, the conversation already has one.
    assert_eq!(now.len(), earlier.len() + 2, "{now:?}");
    assert!(now[earlier.len()].contains("Here are the icons."));
    assert!(now[earlier.len() + 1].contains("thanks"));
    let requests = script.requests();
    assert_eq!(requests[0].system, requests[2].system);
    assert_eq!(requests[0].tools.len(), requests[2].tools.len());
}

#[sqlx::test]
async fn one_turn_at_a_time_and_a_stop_is_heard_between_steps(pool: PgPool) {
    let script = Script::slow(vec![], Duration::from_millis(300));
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    script.replace(vec![calls("icons", json!({})), answers("never reached")]);

    let sent = send(address, &who, id, "look around").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().unwrap();
    let again = send(address, &who, id, "and this").await;
    assert_eq!(again.status, 409, "{}", again.body);

    let path = format!("/api/chat/turns/{turn}/stop");
    let stopped = common::request(address, "POST", &path, &[&who], None).await;
    assert_eq!(stopped.status, 202, "{}", stopped.body);
    let detail = finished(address, &who, turn).await;
    assert_eq!(detail["turn"]["state"], "stopped", "{detail}");
    assert_eq!(script.requests().len(), 1, "stopped before the second call");
    let late = common::request(address, "POST", &path, &[&who], None).await;
    assert_eq!(late.status, 409, "{}", late.body);
}
