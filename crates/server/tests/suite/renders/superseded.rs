//! A preview of a revision the project has moved past retires itself when its
//! turn comes (#542): edits arrive faster than renders finish, and a picture
//! of a document that no longer exists is work for nobody — and one already
//! drawing it is stopped when the next is asked for (#660).

use std::time::Duration;

use scorsese_server::Files;
use scorsese_server::jobs::State;
use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::cancelled::{job_of, long, when, worker as on_queue};
use super::job::finished;
use super::previews::worker;
use super::{call, card, common, member, stored};

#[sqlx::test]
async fn a_preview_of_a_revision_the_project_moved_past_is_never_rendered(pool: PgPool) {
    let files = common::files("preview-superseded");
    let (address, _) = common::serve_with(pool.clone(), files.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let (_, asked) = call(
        address,
        &cookie,
        "POST",
        &format!("/api/projects/{id}/previews"),
        Some(json!({ "resolution": "64x36" })),
    )
    .await;
    let edited = card(|document| document["assets"][0]["color"] = json!("#993366"));
    projects::save(&pool, ana, id, 1, &edited).await.unwrap();

    let _worker = worker(&pool, &files).await;
    let job = finished(&pool, ana, asked["job"]["id"].as_i64().unwrap()).await;
    assert_eq!(job.state, State::Done, "{:?}", job.error);
    assert_eq!(job.result.unwrap()["superseded"], true);
    assert_eq!(super::rows(&pool).await, 0, "nothing rendered");
}

#[sqlx::test]
async fn a_new_preview_stops_the_one_drawing_an_older_revision(pool: PgPool) {
    let files = common::files("cancel-preview");
    let (address, state) = common::serve_with(pool.clone(), files.clone()).await;
    let _worker = on_queue(&pool, &files, &state).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &long(36_000)).await;
    let path = format!("/api/projects/{id}/previews");
    let ask = Some(json!({ "resolution": "64x36" }));
    let (_, first) = call(address, &cookie, "POST", &path, ask.clone()).await;
    let first = job_of(&first);
    when(&pool, ana, first, State::Running).await;
    drawing(&files, first).await;

    let edited = card(|document| document["assets"][0]["color"] = json!("#993366"));
    projects::save(&pool, ana, id, 1, &edited).await.unwrap();
    let (_, second) = call(address, &cookie, "POST", &path, ask).await;
    let second = job_of(&second);
    assert_ne!(first, second);

    when(&pool, ana, first, State::Cancelled).await;
    when(&pool, ana, second, State::Done).await;
}

/// Once job `id` is past its superseded check and drawing — the moment its
/// scratch folder is laid out (#688).
///
/// `Running` is not that moment: a job is `running` from its claim, and only
/// then asks whether the project has moved on. An edit landing in between
/// retires it as superseded before it draws a frame — correct, and not the
/// case this test is about — so the edit waits for the drawing to begin.
async fn drawing(files: &Files, id: i64) {
    let scratch = files.renders.work(id);
    for _ in 0..600 {
        if scratch.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("job {id} never began drawing");
}
