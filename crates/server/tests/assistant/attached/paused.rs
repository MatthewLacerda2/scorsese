//! Attachments sent to a turn paused on a question or a quote: the media are
//! imported and the resumed turn is told, as a new one is.

use scorsese_server::library::Kind;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::{Script, answers, calls, finished, member, project, scripted};
use super::{heard, kept, send_with};

#[sqlx::test]
async fn a_turn_paused_on_a_question_hears_the_attachments_as_it_resumes(pool: PgPool) {
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
    let said = heard(&script, 1);
    assert!(
        said.contains("logo.png"),
        "the resumed turn is told: {said}"
    );
    assert!(said.contains("this one"), "{said}");
}

#[sqlx::test]
async fn a_turn_paused_on_a_quote_hears_the_attachments_written_instead(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, super::super::quotes::narrated()).await;
    script.replace(vec![calls("generate", json!({ "project": id }))]);
    let paused = super::super::exchange(address, &who, id, "narrate it").await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");

    script.replace(vec![answers("Using the logo instead.")]);
    let logo = kept(&state, ana, "logo.png", Kind::Image).await;
    let sent = send_with(address, &who, id, "use this instead", json!([logo])).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;
    let said = heard(&script, 1);
    assert!(
        said.contains("logo.png"),
        "the resumed turn is told: {said}"
    );
    assert!(said.contains("use this instead"), "{said}");
}
