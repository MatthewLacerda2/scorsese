//! A user's templates (#546): saved from one project by the tools, inserted
//! into another, kept to their owner, and holding their library files.

#[path = "../common/mod.rs"]
mod common;

mod files;
mod tools;

use scorsese_core::Project;
use scorsese_server::accounts::{sessions, users};
use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use scorsese_server::projects;
use scorsese_server::tools::Client;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

pub(crate) const SHOT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// An account whose library holds [`SHOT`], and its browser's `Cookie:` line.
pub(crate) async fn member(pool: &PgPool, email: &str) -> (UserId, String) {
    let user = users::create(pool, email, "password one")
        .await
        .expect("an account");
    common::hold(pool, user, SHOT).await;
    let cookie = sessions::open(pool, user).await.expect("a session opens");
    (user, format!("Cookie: scorsese_session={cookie}"))
}

/// A project with an intro in it — the shot on `v1`, a title over it on `v2` —
/// and its id.
pub(crate) async fn episode(pool: &PgPool, user: UserId) -> i64 {
    let mut document = serde_json::to_value(Project::new("Episode 1", Default::default()))
        .expect("a new project serialises");
    document["assets"] = json!([
        { "id": "shot", "kind": "video", "path": format!("assets/{SHOT}.mp4"), "sha256": SHOT,
          "media": { "duration_seconds": 10.0 } },
        { "id": "title", "kind": "text", "text": "EPISODE" }
    ]);
    document["tracks"] = json!([
        { "id": "v1", "kind": "video", "clips": [
            { "id": "c-shot", "asset": "shot", "start": 90, "duration": 120 } ] },
        { "id": "v2", "kind": "video", "clips": [
            { "id": "c-title", "asset": "title", "start": 120, "duration": 60 } ] }
    ]);
    let project = Project::from_json(&document.to_string()).expect("a project");
    projects::create(pool, user, &project)
        .await
        .expect("stored")
        .id
}

/// An empty project of `user`'s, and its id.
pub(crate) async fn empty(pool: &PgPool, user: UserId) -> i64 {
    let project = Project::new("Episode 2", Default::default());
    projects::create(pool, user, &project)
        .await
        .expect("stored")
        .id
}

/// Call `name` for `user` as web MCP would; the reply's words, or the refusal.
pub(crate) async fn call(
    state: &AppState,
    user: UserId,
    name: &str,
    arguments: Value,
) -> Result<String, String> {
    state
        .tools
        .call(user, Client::External, name, &arguments)
        .await
        .map(|reply| reply.parts[0].text.clone())
}

/// Save the intro — both clips — as `name`, for `user`.
pub(crate) async fn save_intro(
    state: &AppState,
    user: UserId,
    project: i64,
    name: &str,
) -> Result<String, String> {
    let arguments = json!({ "project": project, "clips": ["c-shot", "c-title"], "name": name });
    call(state, user, "template_save", arguments).await
}

/// `user`'s one template's id.
pub(crate) async fn only_template(state: &AppState, user: UserId) -> i64 {
    let listed = scorsese_server::templates::list(&state.pool, user)
        .await
        .expect("listed");
    assert_eq!(listed.len(), 1, "{listed:?}");
    listed[0].id
}
