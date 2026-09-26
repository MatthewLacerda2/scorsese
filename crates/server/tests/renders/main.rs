//! Finished renders (`src/renders`): asked for over HTTP, made by the job
//! queue from a stored project, downloaded in ranges, and evicted by the
//! 48-hour rule — never while somebody has the file open.
//!
//! The projects rendered here are a colour card a few frames long at a
//! postage-stamp size: the pipeline end to end, in a fraction of a second.

#[path = "../common/mod.rs"]
mod common;

mod evict;
mod job;
mod request;
mod settings;

use std::net::SocketAddr;
use std::path::Path;

use scorsese_core::Project;
use scorsese_server::accounts::{sessions, users};
use scorsese_server::db::{self, UserId};
use scorsese_server::projects;
use scorsese_server::renders::{RenderCache, RenderView, Settings, store};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

/// A new account, and a logged-in browser's `Cookie:` line for it.
async fn member(pool: &PgPool, email: &str) -> (UserId, String) {
    let user = users::create(pool, email, "password one")
        .await
        .expect("the account is created");
    let cookie = sessions::open(pool, user).await.expect("a session opens");
    (user, format!("Cookie: scorsese_session={cookie}"))
}

/// A project six frames long: one colour card, with whatever `extra` adds
/// to the document.
fn card(extra: impl FnOnce(&mut Value)) -> Project {
    let mut document = serde_json::to_value(Project::new("card", Default::default()))
        .expect("a new project serialises");
    document["assets"] = json!([{ "id": "bg", "kind": "color", "color": "#336699" }]);
    document["tracks"] = json!([{
        "id": "v1", "kind": "video",
        "clips": [{ "id": "c1", "asset": "bg", "start": 0, "duration": 6 }]
    }]);
    extra(&mut document);
    Project::from_json(&document.to_string()).expect("the card is a project")
}

/// `project`, stored as one of `user`'s; its id.
async fn stored(pool: &PgPool, user: UserId, project: &Project) -> i64 {
    projects::create(pool, user, project)
        .await
        .expect("the project is stored")
        .id
}

/// A row and a file for a render of `project` under `key`, as a finished job
/// leaves them — without rendering anything.
async fn kept(
    pool: &PgPool,
    cache: &RenderCache,
    user: UserId,
    project: i64,
    key: &str,
) -> RenderView {
    let settings = Settings::from_ask(&Default::default()).expect("the defaults are allowed");
    let relative = RenderCache::relative(user, project, key, settings.extension());
    let path = cache.absolute(&relative);
    std::fs::create_dir_all(path.parent().expect("a file has a folder"))
        .expect("the test setup works");
    std::fs::write(&path, b"\0\0\0\x20ftypisom and then some").expect("the test setup works");
    let mut tx = db::scoped(pool, user).await.expect("the test setup works");
    let view = store::insert(
        &mut tx,
        project,
        key,
        &settings,
        &relative.to_string_lossy(),
        26,
    )
    .await
    .expect("the render is recorded");
    tx.commit().await.expect("the test setup works");
    view
}

/// Make render `id` look unused for `hours`.
async fn idle_for(pool: &PgPool, id: i64, hours: i64) {
    sqlx::query(
        "UPDATE renders SET last_used_at = now() - make_interval(hours => $2) WHERE id = $1",
    )
    .bind(id)
    .bind(i32::try_from(hours).expect("the test setup works"))
    .execute(pool)
    .await
    .expect("the test setup works");
}

/// How many render rows there are, everybody's.
async fn rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM renders")
        .fetch_one(pool)
        .await
        .expect("the test setup works")
}

/// Whether render `view`'s file is on disk.
fn on_disk(cache: &RenderCache, user: UserId, view: &RenderView) -> bool {
    let relative = RenderCache::relative(user, view.project, &view.key, "mp4");
    cache.absolute(Path::new(&relative)).is_file()
}

/// `method path` as `who`, with a JSON body: the status and the JSON back.
async fn call(
    address: SocketAddr,
    who: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let response = common::request(address, method, path, &[who], body.as_ref()).await;
    let json = serde_json::from_str(&response.body).unwrap_or(Value::Null);
    (response.status, json)
}
