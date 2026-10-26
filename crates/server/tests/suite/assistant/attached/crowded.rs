//! A conversation whose replayed pictures would pass what one request may
//! carry compacts at the start of its next turn, however few tokens it holds
//! (#1562): the vendor refuses a request with too many images.

use scorsese_server::library::Kind;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::compaction::summarising;
use super::super::{Script, answers, finished, member, project, scripted};
use super::{kept, send_with};

/// Pictures a request replays, its tool results' included.
fn pictures(script: &Script, n: usize) -> usize {
    script
        .messages(n)
        .concat()
        .matches(r#""type":"image""#)
        .count()
}

#[sqlx::test]
async fn too_many_pictures_to_replay_compact_the_conversation(pool: PgPool) {
    let mut replies: Vec<_> = (0..8).map(|_| answers("Seen.")).collect();
    replies.push(answers("SUMMARY: eight sets of stills."));
    replies.push(answers("Seen the ninth."));
    let script = Script::new(replies);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let mut items = Vec::new();
    for n in 0..8 {
        items.push(kept(&state, ana, &format!("still{n}.png"), Kind::Image).await);
    }

    for round in 0..9 {
        let sent = send_with(address, &who, id, &format!("set {round}"), json!(items)).await;
        assert_eq!(sent.status, 202, "{}", sent.body);
        finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;
    }
    assert!(
        (0..8).all(|n| !summarising(&script, n)),
        "64 pictures replay as they are"
    );
    assert_eq!(pictures(&script, 7), 64);
    assert!(summarising(&script, 8), "the ninth set would make 72");
    assert_eq!(pictures(&script, 9), 8, "the turn opens with its own alone");
}
