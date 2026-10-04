//! The answer resumes the same turn, wherever it comes from.

use super::*;

#[sqlx::test]
async fn a_question_pauses_the_turn_free_and_the_answer_resumes_it(pool: PgPool) {
    let script = Script::new(vec![calls("ask_user", question()), answers("Faded.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let paused = asked(address, &who, id, "add the music").await;
    let turn = paused["turn"]["id"].as_i64().unwrap();
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");
    let card = &paused["turn"]["questions"][0];
    assert_eq!(card["options"], json!(["loop it", "fade it"]), "{card}");
    assert!(card["answer"].is_null());
    assert!(paused["tools"].as_array().unwrap().is_empty(), "not a tool");
    let tools: Vec<String> = script.requests()[0]
        .tools
        .iter()
        .map(|t| t.name.clone())
        .collect();
    assert_eq!(tools.last().map(String::as_str), Some("ask_user"));

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(script.requests().len(), 1, "nothing runs while it waits");
    assert_eq!(
        paused["turn"]["charged_micros"], CHARGED,
        "one call, no more"
    );

    let resumed = answer(address, &who, turn, "fade it").await;
    assert_eq!(resumed.status, 202, "{}", resumed.body);
    assert_eq!(resumed.json()["id"], turn, "the same turn");
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    assert_eq!(done["turn"]["answer"], "Faded.");
    assert_eq!(done["turn"]["questions"][0]["answer"], "fade it");
    assert_eq!(done["turn"]["charged_micros"], 2 * CHARGED);
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_calls WHERE turn_id = $1")
        .bind(turn)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(calls, 2, "the resumed call is recorded too");

    let (before, after) = (script.messages(0), script.messages(1));
    assert_eq!(
        after[..before.len()],
        before[..],
        "append-only, byte for byte"
    );
    assert!(after[before.len()].contains("\"thinking\""), "{after:?}");
    let result = &script.parsed(1)[before.len() + 1]["content"][0];
    assert_eq!(result["type"], "tool_result", "{result}");
    assert_eq!(result["content"][0]["text"], "The person answered: fade it");

    let again = answer(address, &who, turn, "loop it").await;
    assert_eq!(again.status, 400, "answered once: {}", again.body);
}

#[sqlx::test]
async fn on_gemini_a_typed_message_is_the_answer(pool: PgPool) {
    let script = Script::new(vec![calls("ask_user", question()), answers("Looped.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = new_project(&pool, ana, json!([])).await;
    let paused = asked(address, &who, id, "add the music").await;
    assert_eq!(paused["turn"]["model"], "gemini-3.8-flash");
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");
    let turn = paused["turn"]["id"].clone();

    let typed = asked(address, &who, id, "loop it, but quieter").await;
    assert_eq!(
        typed["turn"]["id"], turn,
        "a message answers, it starts nothing"
    );
    assert_eq!(typed["turn"]["prompt"], "add the music");
    assert_eq!(typed["turn"]["answer"], "Looped.");
    assert_eq!(
        typed["turn"]["questions"][0]["answer"],
        "loop it, but quieter"
    );
    let sent = script.messages(1);
    let result = sent.last().unwrap();
    assert!(
        result.contains("functionResponse") && result.contains("ask_user"),
        "{result}"
    );
    assert!(
        result.contains("The person answered: loop it, but quieter"),
        "{result}"
    );
}
