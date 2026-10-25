//! What the model sees of a message's attachments (#1562): the first eight
//! pictures in the message itself, shrunk; a document's text when it is
//! short; a video, a sound and whatever is past the budget only named.

use scorsese_server::assistant::{MAX_INLINE_CHARS, MAX_PICTURES};
use scorsese_server::library::Kind;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::super::{Script, answers, calls, finished, member, project, scripted};
use super::{heard, kept, send_with, written};

/// The blocks of the person's message in request `n`: its first user
/// message's content.
fn opened(script: &Script, n: usize) -> Vec<Value> {
    script
        .parsed(n)
        .into_iter()
        .find(|message| message["role"] == "user")
        .and_then(|message| message["content"].as_array().cloned())
        .expect("the turn opens with the person's message")
}

/// How many pictures `blocks` carry, a tool result's included.
fn pictures(blocks: &[Value]) -> usize {
    blocks
        .iter()
        .map(|block| match block["type"].as_str() {
            Some("image") => 1,
            Some("tool_result") => pictures(block["content"].as_array().map_or(&[], Vec::as_slice)),
            _ => 0,
        })
        .sum()
}

#[sqlx::test]
async fn pictures_are_shown_a_document_inline_and_video_and_sound_named(pool: PgPool) {
    let script = Script::new(vec![answers("Got them.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let one = kept(&state, ana, "one.png", Kind::Image).await;
    let two = kept(&state, ana, "two.jpg", Kind::Image).await;
    let clip = kept(&state, ana, "clip.mp4", Kind::Video).await;
    let brief = written(
        &state,
        ana,
        "brief.md",
        "# Brief\n\nA sunrise, then coffee.",
    )
    .await;
    let song = kept(&state, ana, "song.mp3", Kind::Audio).await;

    let attached = json!([one, two, clip, brief, song]);
    let sent = send_with(address, &who, id, "make it", attached).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;

    let blocks = opened(&script, 0);
    assert_eq!(blocks[0]["text"], "make it", "the words come first");
    assert_eq!(pictures(&blocks), 2, "{blocks:?}");
    for picture in blocks.iter().filter(|block| block["type"] == "image") {
        assert_eq!(picture["source"]["media_type"], "image/jpeg", "shrunk");
    }
    let inline: Vec<_> = blocks.iter().filter_map(|b| b["text"].as_str()).collect();
    assert!(
        inline
            .iter()
            .any(|text| text.contains("A sunrise, then coffee.")),
        "the markdown is in the message: {inline:?}"
    );
    let said = heard(&script, 0);
    for named in ["clip.mp4", "song.mp3"] {
        let line = said
            .split("\\n")
            .find(|line| line.contains(named) && line.contains("not shown"))
            .unwrap_or_else(|| panic!("{named} is named, not shown: {said}"));
        assert!(!line.contains("library_read"), "{line}");
    }
}

#[sqlx::test]
async fn a_message_shows_eight_pictures_and_names_the_rest(pool: PgPool) {
    let script = Script::new(vec![answers("Twelve.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let mut items = Vec::new();
    for n in 0..12 {
        items.push(kept(&state, ana, &format!("still{n}.png"), Kind::Image).await);
    }

    let sent = send_with(address, &who, id, "all of these", json!(items)).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;

    assert_eq!(pictures(&opened(&script, 0)), MAX_PICTURES);
    let said = heard(&script, 0);
    let past = said.matches("a message shows at most 8 pictures").count();
    assert_eq!(past, 4, "the four past the budget are named: {said}");
    for n in 8..12 {
        assert!(
            said.contains(&format!("still{n}.png” — not shown")),
            "{said}"
        );
    }
}

#[sqlx::test]
async fn a_long_document_is_named_to_read_rather_than_put_in_the_message(pool: PgPool) {
    let script = Script::new(vec![answers("Reading.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let long = "word ".repeat(MAX_INLINE_CHARS / 5 + 1);
    let script_md = written(&state, ana, "script.md", &long).await;

    let sent = send_with(address, &who, id, "read it", json!([script_md])).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;

    let said = heard(&script, 0);
    assert!(said.contains("library_read"), "{said}");
    assert!(!said.contains(&long[..1000]), "the text is not sent whole");
}

#[sqlx::test]
async fn a_turn_paused_on_a_question_is_shown_the_pictures_it_is_answered_with(pool: PgPool) {
    let question = json!({ "question": "Which logo?", "options": ["old", "new"] });
    let script = Script::new(vec![calls("ask_user", question), answers("Using it.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let paused = super::super::exchange(address, &who, id, "add the logo").await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");

    let logo = kept(&state, ana, "logo.png", Kind::Image).await;
    let sent = send_with(address, &who, id, "this one", json!([logo])).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;
    let last = script.parsed(1).pop().expect("the answer is sent");
    let content = last["content"].as_array().expect("blocks");
    assert_eq!(pictures(content), 1, "{last}");
}
