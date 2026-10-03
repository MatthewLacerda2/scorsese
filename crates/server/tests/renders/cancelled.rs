//! Stopping a render (#660): a waiting one is cancelled where it stands, a
//! running one stops within a frame and keeps nothing, and only its owner may
//! ask. A newer preview stopping an older one is in `superseded.rs`.
//!
//! "Running" here is a colour card twenty minutes long at a postage-stamp
//! size: long enough that the cancel always lands mid-render, and stopped
//! within a second of it.

use std::time::Duration;

use scorsese_server::Files;
use scorsese_server::db::{self, UserId};
use scorsese_server::http::AppState;
use scorsese_server::jobs::{JobView, Registry, State, kinds, store, work};
use scorsese_server::renders::job;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::{call, card, common, member, stored};

/// A card of `frames` frames.
pub(super) fn long(frames: u64) -> scorsese_core::Project {
    card(|document| document["tracks"][0]["clips"][0]["duration"] = json!(frames))
}

/// A worker for renders and previews on the server's own queue — the one
/// whose flags `POST /api/jobs/{id}/cancel` trips, as in production.
pub(super) async fn worker(pool: &PgPool, files: &Files, state: &AppState) -> watch::Sender<bool> {
    let members = db::member_pool(pool)
        .await
        .expect("the member pool connects");
    let handler = || {
        job::handler(
            files.renders.clone(),
            files.tools.clone(),
            files.storage.clone(),
        )
    };
    let registry = Registry::new()
        .register(kinds::RENDER, handler())
        .register(kinds::PREVIEW, handler());
    let (stop, stopping) = watch::channel(false);
    tokio::spawn(work(members, registry, state.jobs.clone(), stopping));
    stop
}

/// Job `id` once it is in `state`.
pub(super) async fn when(pool: &PgPool, user: UserId, id: i64, state: State) -> JobView {
    let members = db::member_pool(pool)
        .await
        .expect("the member pool connects");
    for _ in 0..600 {
        let job = store::get(&members, user, id)
            .await
            .expect("the job reads")
            .expect("the job is theirs");
        if job.state == state {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("job {id} never reached {state:?}");
}

/// The job id an ask answered with.
pub(super) fn job_of(asked: &Value) -> i64 {
    asked["job"]["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("no job in {asked}"))
}

#[sqlx::test]
async fn a_running_render_stops_is_cancelled_and_keeps_nothing(pool: PgPool) {
    let files = common::files("cancel-running");
    let (address, state) = common::serve_with(pool.clone(), files.clone()).await;
    let _worker = worker(&pool, &files, &state).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &long(36_000)).await;
    let ask = Some(json!({ "resolution": "64x36" }));
    let (_, asked) = call(
        address,
        &cookie,
        "POST",
        &format!("/api/projects/{id}/renders"),
        ask,
    )
    .await;
    let job = job_of(&asked);
    when(&pool, ana, job, State::Running).await;

    let (status, said) = call(
        address,
        &cookie,
        "POST",
        &format!("/api/jobs/{job}/cancel"),
        None,
    )
    .await;
    assert_eq!(status, 200, "{said}");
    let stopped = when(&pool, ana, job, State::Cancelled).await;
    let why = stopped.error.unwrap();
    assert!(
        why.contains("cancelled after") && why.contains("of 36000"),
        "{why}"
    );
    assert_eq!(super::rows(&pool).await, 0, "nothing kept");
    let scratch = files.renders.work(0).parent().unwrap().to_path_buf();
    assert!(
        std::fs::read_dir(scratch).unwrap().next().is_none(),
        "no file left behind"
    );
}

#[sqlx::test]
async fn a_waiting_render_is_cancelled_where_it_stands_by_its_owner_only(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("cancel-waiting")).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let (_, bob) = member(&pool, "bob@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let (_, asked) = call(
        address,
        &cookie,
        "POST",
        &format!("/api/projects/{id}/renders"),
        Some(json!({})),
    )
    .await;
    let job = job_of(&asked);
    let cancel = format!("/api/jobs/{job}/cancel");

    assert_eq!(
        call(address, &bob, "POST", &cancel, None).await.0,
        404,
        "not bob's"
    );
    let (status, said) = call(address, &cookie, "POST", &cancel, None).await;
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["state"], "cancelled");
    assert_eq!(said["error"], "cancelled before it started");
    let (status, again) = call(address, &cookie, "POST", &cancel, None).await;
    assert_eq!(
        (status, &again["state"]),
        (200, &json!("cancelled")),
        "asking twice is fine"
    );

    // A paid generation is never stopped: the provider bills either way.
    let members = db::member_pool(&pool).await.unwrap();
    let mut tx = db::scoped(&members, ana).await.unwrap();
    let shot = store::enqueue(&mut tx, kinds::VEO_SHOT, &json!({}))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let (status, said) = call(
        address,
        &cookie,
        "POST",
        &format!("/api/jobs/{}/cancel", shot.id),
        None,
    )
    .await;
    assert_eq!(status, 409, "{said}");
}
