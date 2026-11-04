//! A server whose job worker reaches the mock vendors, and waiting for its
//! jobs to settle.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use scorsese_server::Files;
use scorsese_server::generations::Timing;
use scorsese_server::http::AppState;
use scorsese_server::jobs::elevenlabs::ElevenLabs;
use scorsese_server::jobs::{self, kinds};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::super::common;
use super::Mock;

/// A server whose job worker reaches `mock`: its address, its state, and
/// what keeps the worker running — dropping it stops the worker.
pub(in super::super) async fn serve(
    pool: &PgPool,
    mock: &Mock,
) -> (SocketAddr, AppState, watch::Sender<bool>) {
    serve_on(pool, mock, ElevenLabs::default()).await
}

/// [`serve`], on an ElevenLabs plan that allows what `plan` says.
pub(in super::super) async fn serve_on(
    pool: &PgPool,
    mock: &Mock,
    plan: ElevenLabs,
) -> (SocketAddr, AppState, watch::Sender<bool>) {
    let (address, state, files) = serve_idle(pool).await;
    let stop = work(pool, &state, &files, mock, plan);
    (address, state, stop)
}

/// A server with no job worker yet, so a job it queues waits for [`work`]:
/// its address, its state, and the files a worker needs.
pub(in super::super) async fn serve_idle(pool: &PgPool) -> (SocketAddr, AppState, Files) {
    let files = common::files("mcp-paid");
    let (address, state) = common::serve_with(pool.clone(), files.clone()).await;
    (address, state, files)
}

/// A job worker for [`serve_idle`]'s server, reaching `mock` on `plan`: what
/// keeps it running — dropping it stops the worker.
pub(in super::super) fn work(
    pool: &PgPool,
    state: &AppState,
    files: &Files,
    mock: &Mock,
    plan: ElevenLabs,
) -> watch::Sender<bool> {
    let timing = Timing {
        poll_every: Duration::from_millis(20),
        patience: Duration::from_secs(20),
        batch_every: Duration::from_millis(20),
        batch_patience: Duration::from_secs(20),
    };
    let registry = kinds::with_vendors(files, Arc::new(mock.clone()), timing, plan);
    let (stop, stopping) = watch::channel(false);
    let worker = tokio::spawn(jobs::work(
        state.pool.clone(),
        registry,
        state.jobs.clone(),
        stopping,
    ));
    common::release_at_teardown(pool, &state.pool, [worker.abort_handle()]);
    stop
}

/// Wait until every job of `user`'s has finished; their states.
pub(in super::super) async fn settled(
    state: &AppState,
    user: scorsese_server::db::UserId,
) -> Vec<String> {
    for _ in 0..400 {
        let all = jobs::store::list(&state.pool, user)
            .await
            .expect("the jobs list");
        let busy = all.iter().any(|job| {
            matches!(job.state, jobs::State::Waiting | jobs::State::Running)
                && job.kind != "thumbnail"
        });
        if !busy {
            return all
                .iter()
                .filter(|job| job.kind != "thumbnail")
                .map(|job| format!("{:?}", job.state).to_lowercase())
                .collect();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the jobs never finished");
}
