//! Files attached to a message (#1534): the chat's tray sends their library
//! ids with the prompt, the media among them are brought into the project
//! before the turn starts, and the model is told which files came with this
//! message — a document as one to read, never imported — so it does not go
//! looking for them. A turn paused on a question or a quote hears them as it
//! resumes (`paused`).

use std::path::PathBuf;

use scorsese_render::testing::generate;
use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use scorsese_server::library::{Arrival, Kind};
use scorsese_server::projects;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, common, finished, member, project, scripted};

mod paused;
mod refused;

/// `name` in `user`'s library: a red square for a picture, a line of text otherwise.
pub(super) async fn kept(state: &AppState, user: UserId, name: &str, kind: Kind) -> i64 {
    let scratch = common::scratch::dir("attached");
    let extension = name.rsplit_once('.').map(|(_, e)| e).expect("an extension");
    let file: PathBuf = scratch.join(name);
    match kind {
        Kind::Image => generate(
            &common::tools(),
            &file,
            &["-f", "lavfi", "-i", "color=c=red:s=16x16", "-frames:v", "1"],
        ),
        _ => std::fs::write(&file, "Scene one: a door opens.").expect("the test setup holds"),
    }
    let arrival = Arrival {
        file,
        name: name.to_owned(),
        kind,
        extension: extension.to_owned(),
        announced: None,
        brief_hash: None,
        words: None,
    };
    state
        .library
        .admit(user, arrival)
        .await
        .expect("the test setup holds")
        .id
}

/// `prompt` sent about `project` with `attachments`: the response.
pub(super) async fn send_with(
    address: std::net::SocketAddr,
    who: &str,
    project: i64,
    prompt: &str,
    attachments: Value,
) -> common::Response {
    let path = format!("/api/projects/{project}/chat");
    let body = json!({ "prompt": prompt, "attachments": attachments });
    common::request(address, "POST", &path, &[who], Some(&body)).await
}

/// Everything request `n` sent the model, as one text.
pub(super) fn heard(script: &Script, n: usize) -> String {
    script.messages(n).join("\n")
}

#[sqlx::test]
async fn attached_media_is_imported_and_the_model_told_which_files_came(pool: PgPool) {
    let script = Script::new(vec![answers("Got them.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let logo = kept(&state, ana, "logo.png", Kind::Image).await;
    let brief = kept(&state, ana, "brief.txt", Kind::Text).await;

    let sent = send_with(address, &who, id, "use these", json!([logo, brief])).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;

    let document = projects::open(&pool, ana, id)
        .await
        .expect("opens")
        .document;
    let names: Vec<_> = document.assets.iter().map(|a| a.id.to_string()).collect();
    assert_eq!(names.len(), 1, "the picture alone is imported: {names:?}");

    let said = heard(&script, 0);
    assert!(said.contains("logo.png"), "the picture is named: {said}");
    assert!(said.contains(&format!("asset `{}`", names[0])), "{said}");
    assert!(said.contains("brief.txt"), "the document is named: {said}");
    assert!(
        said.contains("library_read"),
        "a document is to be read: {said}"
    );
    assert!(
        said.contains("use these"),
        "the person's words still reach it"
    );
}

#[sqlx::test]
async fn a_message_without_attachments_says_nothing_of_them(pool: PgPool) {
    let script = Script::new(vec![answers("Hi.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let sent = send_with(address, &who, id, "hello", json!([])).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().expect("an id")).await;
    assert!(
        !heard(&script, 0).contains("attached"),
        "{}",
        heard(&script, 0)
    );
}

#[sqlx::test]
async fn a_file_not_in_the_senders_library_is_refused_before_anything_runs(pool: PgPool) {
    let script = Script::new(vec![answers("Never.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let (bia, _) = member(&pool, "bia@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let theirs = kept(&state, bia, "logo.png", Kind::Image).await;

    for attachments in [json!([theirs]), json!([999_999])] {
        let sent = send_with(address, &who, id, "use it", attachments).await;
        assert_eq!(sent.status, 400, "{}", sent.body);
        assert!(sent.body.contains("library"), "{}", sent.body);
    }
    assert!(script.requests().is_empty(), "no turn started");
    let document = projects::open(&pool, ana, id)
        .await
        .expect("opens")
        .document;
    assert!(document.assets.is_empty(), "nothing was imported");
}
