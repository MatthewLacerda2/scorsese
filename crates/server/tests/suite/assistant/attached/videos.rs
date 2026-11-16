//! An attached video, as the model sees it (#1563): one sheet of frames
//! chosen across it, counted as one picture of the message's eight, and its
//! transcript beside it when the library has heard it — never a transcription
//! started for it.

use scorsese_core::words::{Word, Words};
use scorsese_server::library::Kind;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::{Script, answers, finished, member, project, scripted};
use super::seen::{opened, pictures};
use super::{heard, heard_video, kept, send_with};

#[sqlx::test]
async fn a_video_is_shown_as_one_sheet_and_said_to_have_no_transcript(pool: PgPool) {
    let script = Script::new(vec![answers("Seen it.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let clip = kept(&state, ana, "clip.mp4", Kind::Video).await;

    let sent = send_with(address, &who, id, "cut this", json!([clip])).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;

    let blocks = opened(&script, 0);
    assert_eq!(
        pictures(&blocks),
        1,
        "one sheet, not a picture a frame: {blocks:?}"
    );
    let sheet = blocks
        .iter()
        .find(|b| b["type"] == "image")
        .expect("a sheet");
    assert_eq!(sheet["source"]["media_type"], "image/jpeg", "shrunk");
    let said = heard(&script, 0);
    let line = said
        .split("\\n")
        .find(|line| line.contains("clip.mp4") && line.contains("a video"))
        .unwrap_or_else(|| panic!("the video is named: {said}"));
    assert!(line.contains("one sheet of 3 frames"), "{line}");
    assert!(line.contains("no transcript"), "{line}");
}

#[sqlx::test]
async fn a_heard_videos_transcript_is_beside_its_sheet(pool: PgPool) {
    let script = Script::new(vec![answers("Heard it.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let words = Words {
        words: vec![
            Word {
                text: "Hello".into(),
                start: 0.2,
                end: 0.4,
            },
            Word {
                text: "there.".into(),
                start: 0.5,
                end: 0.8,
            },
        ],
    };
    let talk = heard_video(&state, ana, "talk.mp4", words).await;

    let sent = send_with(address, &who, id, "caption it", json!([talk])).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;

    let blocks = opened(&script, 0);
    assert_eq!(pictures(&blocks), 1, "{blocks:?}");
    let texts: Vec<&str> = blocks.iter().filter_map(|b| b["text"].as_str()).collect();
    let transcript = texts
        .iter()
        .find(|text| text.contains("talk.mp4") && text.contains("0:00.2 Hello there."))
        .unwrap_or_else(|| panic!("the transcript is in the message: {texts:?}"));
    assert!(transcript.contains("same times"), "{transcript}");
    let said = heard(&script, 0);
    assert!(said.contains("with its transcript"), "{said}");
}

#[sqlx::test]
async fn a_video_past_the_eight_pictures_is_named_only(pool: PgPool) {
    let script = Script::new(vec![answers("Many.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let mut items = Vec::new();
    for n in 0..8 {
        items.push(kept(&state, ana, &format!("still{n}.png"), Kind::Image).await);
    }
    items.push(kept(&state, ana, "late.mp4", Kind::Video).await);

    let sent = send_with(address, &who, id, "all of it", json!(items)).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;

    assert_eq!(pictures(&opened(&script, 0)), 8);
    let said = heard(&script, 0);
    assert!(
        said.contains("late.mp4” — not shown to you: a message shows at most 8 pictures"),
        "{said}"
    );
}
