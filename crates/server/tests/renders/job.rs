//! The render job end to end: asked for over HTTP, rendered by the worker
//! through `scorsese-render`, kept, answered from the cache, downloaded.

use std::time::Duration;

use scorsese_server::Files;
use scorsese_server::db::UserId;
use scorsese_server::events::Events;
use scorsese_server::jobs::{JobView, Queue, Registry, State, kinds, store, work};
use scorsese_server::renders::job;
use serde_json::json;
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::{call, card, common, member, stored};

/// A worker running only the render handler, on `files`, until the sender
/// is dropped or says stop.
async fn worker(pool: &PgPool, files: &Files) -> watch::Sender<bool> {
    let members = scorsese_server::db::member_pool(pool)
        .await
        .expect("the member pool connects");
    let handler = job::handler(
        files.renders.clone(),
        files.tools.clone(),
        files.storage.clone(),
    );
    let registry = Registry::new().register(kinds::RENDER, handler);
    let (stop, stopping) = watch::channel(false);
    tokio::spawn(work(members, registry, Queue::new(Events::new()), stopping));
    stop
}

/// Job `id` once it has finished, however it finished.
async fn finished(pool: &PgPool, user: UserId, id: i64) -> JobView {
    let members = scorsese_server::db::member_pool(pool)
        .await
        .expect("the member pool connects");
    for _ in 0..600 {
        let job = store::get(&members, user, id)
            .await
            .expect("the job reads")
            .expect("the job is theirs");
        if matches!(job.state, State::Done | State::Failed) {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("render job {id} never finished");
}

#[sqlx::test]
async fn a_stored_project_is_rendered_kept_and_downloaded_in_ranges(pool: PgPool) {
    let files = common::files("job-render");
    let cache = files.renders.clone();
    let _worker = worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let path = format!("/api/projects/{id}/renders");
    let ask = json!({ "resolution": "64x36" });

    let (status, asked) = call(address, &cookie, "POST", &path, Some(ask.clone())).await;
    assert_eq!(status, 202, "{asked}");
    let job = finished(&pool, ana, asked["job"]["id"].as_i64().unwrap()).await;
    assert_eq!(job.state, State::Done, "{:?}", job.error);
    let result = job.result.unwrap();

    let (status, again) = call(address, &cookie, "POST", &path, Some(ask)).await;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["render"]["id"], result["render"]);
    assert_eq!(again["render"]["settings"]["resolution"], "64x36");
    assert!(again["render"]["size"].as_i64() > Some(0));

    // An mp4 starts with its `ftyp` box: bytes 4 to 7.
    let file = result["file"].as_str().unwrap();
    let range = common::request(address, "GET", file, &[&cookie, "Range: bytes=4-7"], None).await;
    assert_eq!(range.status, 206);
    assert_eq!(range.body, "ftyp");
    assert_eq!(range.header("content-type"), Some("video/mp4"));
    assert_eq!(
        range.header("content-disposition"),
        Some("attachment; filename=\"card.mp4\"")
    );
    assert!(
        range
            .header("content-range")
            .unwrap()
            .starts_with("bytes 4-7/")
    );
    assert!(
        std::fs::read_dir(cache.work(0).parent().unwrap())
            .unwrap()
            .next()
            .is_none(),
        "scratch cleaned"
    );
}

#[sqlx::test]
async fn a_synthesised_sound_without_its_bake_is_refused_by_its_recipe(pool: PgPool) {
    let files = common::files("job-synth");
    let _worker = worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let project = card(|document| {
        let assets = document["assets"].as_array_mut().unwrap();
        assets
            .push(json!({ "id": "theme", "kind": "synth_audio", "recipe": "recipes/theme.json" }));
        let tracks = document["tracks"].as_array_mut().unwrap();
        tracks.push(json!({ "id": "a1", "kind": "audio",
            "clips": [{ "id": "m", "asset": "theme", "start": 0, "duration": 6 }] }));
    });
    let id = stored(&pool, ana, &project).await;

    let (_, asked) = call(
        address,
        &cookie,
        "POST",
        &format!("/api/projects/{id}/renders"),
        Some(json!({})),
    )
    .await;
    let job = finished(&pool, ana, asked["job"]["id"].as_i64().unwrap()).await;

    assert_eq!(job.state, State::Failed);
    let error = job.error.unwrap();
    for says in ["`theme`", "recipes/theme.json", "#560"] {
        assert!(error.contains(says), "{says}: {error}");
    }
    assert_eq!(super::rows(&pool).await, 0);
}
