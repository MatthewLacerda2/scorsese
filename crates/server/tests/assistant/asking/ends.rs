//! A question asked beside a tool, or set aside by Stop.

use super::*;

#[sqlx::test]
async fn asked_beside_a_tool_it_is_refused_and_the_tool_runs(pool: PgPool) {
    let mut both = calls("ask_user", question());
    let other = calls("project_describe", json!({ "project": 0 })).message;
    let scorsese_providers::chat::Message::Assistant { content } = &mut both.message else {
        unreachable!()
    };
    content.extend(other.parts().iter().cloned());
    let script = Script::new(vec![both, answers("Done without asking.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let done = asked(address, &who, id, "add the music").await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    assert_eq!(done["turn"]["questions"], json!([]));
    assert_eq!(done["tools"][0]["tool"], "project_describe", "{done}");
    let results = script.parsed(1).last().unwrap()["content"].clone();
    assert_eq!(results[0]["is_error"], true, "{results}");
    assert!(
        results[0]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("only call")
    );
}

#[sqlx::test]
async fn stop_sets_the_question_aside_and_the_next_turn_answers_the_call(pool: PgPool) {
    let script = Script::new(vec![calls("ask_user", question())]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let paused = asked(address, &who, id, "add the music").await;
    let turn = paused["turn"]["id"].as_i64().unwrap();
    let path = format!("/api/chat/turns/{turn}/stop");
    let stopped = common::request(address, "POST", &path, &[&who], None).await;
    assert_eq!(stopped.status, 202, "{}", stopped.body);
    let ended = finished(address, &who, turn).await;
    assert_eq!(ended["turn"]["state"], "stopped", "{ended}");
    assert_eq!(answer(address, &who, turn, "loop it").await.status, 400);

    script.replace(vec![answers("Starting over.")]);
    let next = asked(address, &who, id, "never mind, no music").await;
    assert_ne!(next["turn"]["id"], turn);
    let opening = script.parsed(1);
    let result = &opening[opening.len() - 1]["content"][0];
    assert_eq!(result["is_error"], true, "the call is closed: {opening:?}");
}
