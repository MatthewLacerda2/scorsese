//! A conversation past 150K tokens compacts at the start of its next turn
//! (#1438): one summary and the new prompt, nothing older replayed — and a
//! smaller one, or a turn resuming from a pause, is left as it was.

use scorsese_providers::chat::Reply;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::script::CHARGED;
use super::{Script, answers, calls, exchange, finished, member, project, scripted, send};

/// `reply`, reported as having read `size` tokens in all.
fn sized(mut reply: Reply, size: u64) -> Reply {
    reply.usage.cache_read = size - reply.usage.input;
    reply
}

/// The words of a request's message, whichever side wrote it.
fn words(message: &Value) -> String {
    match &message["content"] {
        Value::String(text) => text.clone(),
        content => content.to_string(),
    }
}

#[sqlx::test]
async fn past_the_threshold_the_next_turn_opens_from_one_summary(pool: PgPool) {
    let script = Script::new(vec![
        sized(answers("Here is the script."), 150_001),
        answers("SUMMARY: an intro for Ana's channel, script agreed, all men."),
        answers("Narration added."),
        answers("Louder now."),
    ]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    exchange(address, &who, id, "write the script, all men").await;

    let detail = exchange(address, &who, id, "add the narration").await;
    assert_eq!(detail["turn"]["answer"], "Narration added.", "{detail}");
    assert_eq!(
        detail["turn"]["calls"], 2,
        "the summary is one of its calls"
    );
    assert_eq!(detail["turn"]["charged_micros"], 2 * CHARGED);

    // The summarising call: the whole conversation, then the ask, whose last
    // sentence forbids a tool — with the tools still declared.
    let summarising = script.parsed(1);
    assert!(words(&summarising[0]).contains("write the script, all men"));
    let ask = &summarising.last().unwrap()["content"];
    let ask = ask.as_array().unwrap().last().unwrap()["text"]
        .as_str()
        .unwrap();
    assert!(ask.ends_with("do not call any tool."), "{ask}");
    assert_eq!(script.requests()[1].tools, script.requests()[0].tools);

    // The turn itself: its prompt and the summary, nothing older.
    let opened = script.parsed(2);
    assert_eq!(opened.len(), 2, "{opened:?}");
    assert_eq!(opened[0]["content"][0]["text"], "add the narration");
    assert_eq!(opened[1]["role"], "system");
    assert!(
        words(&opened[1]).contains("SUMMARY: an intro"),
        "{opened:?}"
    );

    // The next turn's history starts at the summary, byte for byte.
    exchange(address, &who, id, "louder").await;
    let (before, now) = (script.messages(2), script.messages(3));
    assert_eq!(now[..before.len()], before[..], "append-only after it");
    assert!(
        !now.concat().contains("write the script"),
        "nothing older: {now:?}"
    );
}

#[sqlx::test]
async fn at_the_threshold_nothing_is_compacted(pool: PgPool) {
    let script = Script::new(vec![
        sized(answers("Here is the script."), 150_000),
        answers("Narration added."),
    ]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    exchange(address, &who, id, "write the script").await;
    exchange(address, &who, id, "add the narration").await;

    assert_eq!(script.requests().len(), 2, "no summarising call");
    let (earlier, now) = (script.messages(0), script.messages(1));
    assert_eq!(now[..earlier.len()], earlier[..], "the cache prefix holds");
}

#[sqlx::test]
async fn a_turn_resumed_from_a_question_is_never_compacted(pool: PgPool) {
    let question = json!({ "question": "Loop it or fade it?", "options": ["loop", "fade"] });
    let script = Script::new(vec![
        sized(calls("ask_user", question), 400_000),
        answers("Faded."),
    ]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let paused = exchange(address, &who, id, "add the music").await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");

    let answered = send(address, &who, id, "fade").await;
    assert_eq!(answered.status, 202, "{}", answered.body);
    let turn = answered.json()["id"].as_i64().unwrap();
    assert_eq!(
        turn,
        paused["turn"]["id"].as_i64().unwrap(),
        "the same turn"
    );
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["answer"], "Faded.", "{done}");

    assert_eq!(script.requests().len(), 2, "no summarising call");
    let (before, after) = (script.messages(0), script.messages(1));
    assert_eq!(after[..before.len()], before[..], "its own loop, intact");
}
