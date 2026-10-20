//! A script the person rewrote by hand, from the editor's Script button
//! (#1445), is news to the assistant: the next turn is told once, so it reads
//! the script again instead of working from what it remembers — a turn paused
//! on a question or a quote as it resumes, too, and never for a write that
//! changed nothing (#1488, #1493).

use std::net::SocketAddr;

use scorsese_server::accounts::sessions;
use scorsese_server::db::UserId;
use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, exchange, finished, member, project, scripted};

/// The words the note carries.
const NOTE: &str = "edited the script themselves";

/// How many times request `n` carries the note.
fn notes_in(script: &Script, n: usize) -> usize {
    script
        .messages(n)
        .iter()
        .map(|text| text.matches(NOTE).count())
        .sum()
}

/// `text` written as the script from the editor, on the revision it is at.
async fn rewrite(
    address: SocketAddr,
    pool: &PgPool,
    user: UserId,
    cookie: &str,
    id: i64,
    text: &str,
) {
    let revision = projects::open(pool, user, id)
        .await
        .expect("opens")
        .summary
        .revision;
    let body = json!({ "revision": revision, "arguments": { "text": text } });
    let path = format!("/api/projects/{id}/tools/script_write");
    let written = common::request(address, "POST", &path, &[cookie], Some(&body)).await;
    assert_eq!(written.status, 200, "{}", written.body);
}

#[sqlx::test]
async fn the_next_turn_is_told_once_that_the_person_rewrote_the_script(pool: PgPool) {
    let script = Script::new(vec![answers("Here is a plan."), answers("Read it.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let cookie = sessions::open(&pool, ana).await.expect("a session opens");
    let cookie = format!("Cookie: scorsese_session={cookie}");
    let id = project(&pool, ana, json!([])).await;

    exchange(address, &who, id, "plan a short film").await;
    assert_eq!(notes_in(&script, 0), 0, "nothing was rewritten yet");

    rewrite(
        address,
        &pool,
        ana,
        &cookie,
        id,
        "# Scene one\nA door opens.",
    )
    .await;
    let stored = projects::open_with_files(&pool, ana, id)
        .await
        .expect("opens")
        .1;
    assert_eq!(stored.get("script.md"), Some("# Scene one\nA door opens."));

    exchange(address, &who, id, "go on").await;
    assert_eq!(
        notes_in(&script, 1),
        1,
        "the turn after the rewrite is told"
    );

    script.replace(vec![answers("Done.")]);
    exchange(address, &who, id, "and the music").await;
    assert_eq!(
        notes_in(&script, 2),
        1,
        "told once: the earlier note is history, and no new one is added"
    );
}

/// Ana's cookie and her bearer, and a new project of hers holding `assets`.
async fn ana_with(pool: &PgPool, assets: serde_json::Value) -> (UserId, String, String, i64) {
    let (ana, who) = member(pool, "ana@example.com", 10).await;
    let cookie = sessions::open(pool, ana).await.expect("a session opens");
    let id = project(pool, ana, assets).await;
    (ana, who, format!("Cookie: scorsese_session={cookie}"), id)
}

/// Ana, with an empty project.
async fn ana(pool: &PgPool) -> (UserId, String, String, i64) {
    ana_with(pool, json!([])).await
}

#[sqlx::test]
async fn a_write_that_changed_nothing_is_not_news(pool: PgPool) {
    let script = Script::new(vec![answers("Here is a plan."), answers("Read it.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who, cookie, id) = ana(&pool).await;
    rewrite(address, &pool, ana, &cookie, id, "A door opens.").await;
    exchange(address, &who, id, "plan a short film").await;

    rewrite(address, &pool, ana, &cookie, id, "A door opens.").await;
    exchange(address, &who, id, "go on").await;
    assert_eq!(notes_in(&script, 1), 0, "the same text, written back");
}

#[sqlx::test]
async fn a_turn_paused_on_a_question_is_told_as_it_resumes(pool: PgPool) {
    let question = json!({ "question": "Loop it or fade it?", "options": ["loop", "fade"] });
    let script = Script::new(vec![calls("ask_user", question), answers("Faded.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who, cookie, id) = ana(&pool).await;
    let paused = exchange(address, &who, id, "add the music").await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");

    rewrite(address, &pool, ana, &cookie, id, "# Scene one").await;
    let typed = super::send(address, &who, id, "fade it").await;
    assert_eq!(typed.status, 202, "{}", typed.body);
    let turn = typed.json()["id"].as_i64().expect("an id");
    finished(address, &who, turn).await;
    assert_eq!(notes_in(&script, 1), 1, "the resumed turn is told");

    script.replace(vec![answers("Done.")]);
    exchange(address, &who, id, "and the titles").await;
    assert_eq!(notes_in(&script, 2), 1, "told once, not again a turn late");
}

#[sqlx::test]
async fn a_turn_paused_on_a_quote_is_told_as_it_resumes(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who, cookie, id) = ana_with(&pool, super::quotes::narrated()).await;
    script.replace(vec![calls("generate", json!({ "project": id }))]);
    let paused = exchange(address, &who, id, "narrate it").await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");

    rewrite(address, &pool, ana, &cookie, id, "# Scene one").await;
    script.replace(vec![answers("Nothing was made.")]);
    let path = format!("/api/chat/turns/{}/quote", paused["turn"]["id"]);
    let body = json!({ "confirm": false });
    let declined = common::request(address, "POST", &path, &[who.as_str()], Some(&body)).await;
    assert_eq!(declined.status, 200, "{}", declined.body);
    let turn = paused["turn"]["id"].as_i64().expect("an id");
    finished(address, &who, turn).await;
    assert_eq!(notes_in(&script, 1), 1, "the resumed turn is told");

    script.replace(vec![answers("Done.")]);
    exchange(address, &who, id, "and the titles").await;
    assert_eq!(notes_in(&script, 2), 1, "told once, not again a turn late");
}
