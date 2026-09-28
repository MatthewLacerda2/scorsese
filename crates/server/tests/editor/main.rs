//! The web editor's route (`src/http/editor.rs`): a hand-edit is one of a few
//! tools, run for the browser's own user, against the revision it was worked
//! out on, and recorded as the editor's.

#[path = "../common/mod.rs"]
mod common;

mod clips;

use std::net::SocketAddr;

use scorsese_core::Project;
use scorsese_server::accounts::{sessions, tokens, users};
use scorsese_server::db::UserId;
use scorsese_server::projects;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

/// A new account, and a logged-in browser's `Cookie:` line for it.
pub(crate) async fn member(pool: &PgPool, email: &str) -> (UserId, String) {
    let user = users::create(pool, email, "password one")
        .await
        .expect("an account");
    let cookie = sessions::open(pool, user).await.expect("a session opens");
    (user, format!("Cookie: scorsese_session={cookie}"))
}

/// A title on `v1` for two seconds: a project with nothing on disk.
pub(crate) async fn titled(pool: &PgPool, user: UserId) -> (i64, i64) {
    let mut document = serde_json::to_value(Project::new("film", Default::default()))
        .expect("a new project serialises");
    document["assets"] = json!([{ "id": "title", "kind": "text", "text": "HELLO" }]);
    document["tracks"] = json!([{ "id": "v1", "kind": "video",
        "clips": [{ "id": "c1", "asset": "title", "start": 0, "duration": 60 }] }]);
    let project = Project::from_json(&document.to_string()).expect("a project");
    let made = projects::create(pool, user, &project)
        .await
        .expect("stored");
    (made.id, made.revision)
}

pub(crate) async fn tool(
    address: SocketAddr,
    who: &str,
    id: i64,
    name: &str,
    body: Value,
) -> common::Response {
    let path = format!("/api/projects/{id}/tools/{name}");
    common::request(address, "POST", &path, &[who], Some(&body)).await
}

#[sqlx::test]
async fn an_edit_lands_names_its_revision_and_is_on_record(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, browser) = member(&pool, "ana@example.com").await;
    let (id, revision) = titled(&pool, ana).await;

    let moved =
        json!({ "revision": revision, "arguments": { "clip": "c1", "start_seconds": 1.0 } });
    let answered = tool(address, &browser, id, "trim_clip", moved).await;
    assert_eq!(answered.status, 200, "{}", answered.body);
    let body = answered.json();
    assert_eq!(body["project"]["revision"], revision + 1);
    assert_eq!(
        body["project"]["document"]["tracks"][0]["clips"][0]["start"],
        30
    );
    assert!(
        body["said"][0]["text"]
            .as_str()
            .is_some_and(|said| said.contains("c1"))
    );

    // Worked out on the revision before: the project moved on, nothing lands.
    let stale = json!({ "revision": revision, "arguments": { "clip": "c1", "scale": 0.5 } });
    let refused = tool(address, &browser, id, "clip_set", stale).await;
    assert_eq!(refused.status, 409, "{}", refused.body);
    let stored = projects::open(&pool, ana, id).await.expect("opens");
    assert_eq!(stored.summary.revision, revision + 1);
    assert!(stored.document.tracks[0].clips[0].keyframes.is_empty());

    let unnamed = json!({ "arguments": { "clip": "c1", "scale": 0.5 } });
    assert_eq!(
        tool(address, &browser, id, "clip_set", unnamed)
            .await
            .status,
        400
    );

    let clients: Vec<(String, String)> =
        sqlx::query_as("SELECT client, tool FROM tool_calls ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("the log reads");
    assert_eq!(
        clients,
        [("editor".to_owned(), "trim_clip".to_owned())],
        "the stale edit was refused before it ran"
    );
}

#[sqlx::test]
async fn a_refused_edit_says_why_and_a_frame_is_a_picture(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, browser) = member(&pool, "ana@example.com").await;
    let (id, revision) = titled(&pool, ana).await;

    let onto = json!({ "revision": revision,
        "arguments": { "asset": "title", "track": "v1", "start_seconds": 1.0, "duration_seconds": 1.0 } });
    let refused = tool(address, &browser, id, "place_clip", onto).await;
    assert_eq!(refused.status, 422, "{}", refused.body);
    assert!(
        refused.json()["error"]
            .as_str()
            .is_some_and(|why| why.contains("nothing was written"))
    );

    let look = json!({ "arguments": { "at": "0s", "resolution": "320x180" } });
    let still = tool(address, &browser, id, "still", look).await;
    assert_eq!(still.status, 200, "{}", still.body);
    let body = still.json();
    assert!(
        body["said"][0]["image"]
            .as_str()
            .is_some_and(|png| !png.is_empty())
    );
    assert!(body["project"].is_null(), "looking changes nothing");
}

#[sqlx::test]
async fn it_is_a_browser_route_for_the_callers_own_projects_and_few_tools(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, browser) = member(&pool, "ana@example.com").await;
    let (_, bob) = member(&pool, "bob@example.com").await;
    let (id, revision) = titled(&pool, ana).await;
    let edit = json!({ "revision": revision, "arguments": { "clip": "c1", "scale": 0.5 } });

    let issued = tokens::issue(&pool, ana, "laptop").await.expect("a token");
    let token = format!("Authorization: Bearer {}", issued.token);
    assert_eq!(
        tool(address, &token, id, "clip_set", edit.clone())
            .await
            .status,
        403
    );
    assert_eq!(
        tool(address, "X-Nobody: 1", id, "clip_set", edit.clone())
            .await
            .status,
        401
    );
    assert_eq!(
        tool(address, &bob, id, "clip_set", edit.clone())
            .await
            .status,
        404
    );
    for outside in ["project_write", "generate", "render", "nonsense"] {
        let response = tool(address, &browser, id, outside, edit.clone()).await;
        assert_eq!(response.status, 404, "{outside}: {}", response.body);
    }
    assert_eq!(
        projects::open(&pool, ana, id)
            .await
            .expect("opens")
            .summary
            .revision,
        revision
    );
}
