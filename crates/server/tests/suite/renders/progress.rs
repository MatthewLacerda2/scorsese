//! How far a running render has got (#698): read back from the job while it
//! runs — as a reload or a poll sees it — and pushed on the owner's event
//! stream as it moves, never written to the job's row.
//!
//! The render is `cancelled.rs`'s twenty-minute card, stopped once it has
//! been seen drawing.

use std::time::Duration;

use futures_util::StreamExt;
use scorsese_server::events::Event;
use scorsese_server::jobs::State;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::cancelled::{job_of, long, when, worker};
use super::{call, common, member, stored};

#[sqlx::test]
async fn a_running_render_says_how_far_it_has_got(pool: PgPool) {
    let files = common::files("progress");
    let (address, state) = common::serve_with(pool.clone(), files.clone()).await;
    let _worker = worker(&pool, &files, &state).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let mut events = Box::pin(state.events.subscribe(ana));
    let id = stored(&pool, ana, &long(36_000)).await;
    let ask = Some(json!({ "resolution": "64x36" }));
    let route = format!("/api/projects/{id}/renders");
    let (_, asked) = call(address, &cookie, "POST", &route, ask).await;
    let job = job_of(&asked);
    let stored = when(&pool, ana, job, State::Running).await;
    assert_eq!(stored.progress, None, "never a column of the row");

    let pushed = tokio::time::timeout(Duration::from_secs(60), async {
        while let Some(event) = events.next().await {
            if let Event::JobProgress { id, progress } = event
                && id == job
                && progress.phase == "drawing"
            {
                return progress;
            }
        }
        panic!("the stream ended");
    })
    .await
    .expect("progress is pushed while it draws");
    assert_eq!(pushed.of, 36_000, "{pushed:?}");

    let read = read_drawing(address, &cookie, job).await;
    assert_eq!(read["of"], 36_000, "{read}");
    assert!(read["percent"].as_u64().is_some_and(|p| p < 100), "{read}");

    let listed = call(address, &cookie, "GET", "/api/jobs", None).await.1;
    assert_eq!(listed[0]["progress"]["phase"], "drawing", "{listed}");

    let cancel = format!("/api/jobs/{job}/cancel");
    call(address, &cookie, "POST", &cancel, None).await;
    let stopped = when(&pool, ana, job, State::Cancelled).await;
    let (_, after) = call(address, &cookie, "GET", &format!("/api/jobs/{job}"), None).await;
    assert_eq!(after["progress"], Value::Null, "a stopped job has none");
    assert_eq!(stopped.progress, None);
}

/// `GET /api/jobs/{job}`'s progress, once it says the render is drawing.
async fn read_drawing(address: std::net::SocketAddr, cookie: &str, job: i64) -> Value {
    for _ in 0..600 {
        let (status, read) = call(address, cookie, "GET", &format!("/api/jobs/{job}"), None).await;
        assert_eq!(status, 200, "{read}");
        if read["progress"]["phase"] == "drawing" {
            return read["progress"].clone();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("job {job} was never read drawing");
}
