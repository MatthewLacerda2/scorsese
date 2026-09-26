//! The job queue (`src/jobs`): claiming, recovery, the worker, and the live
//! stream a job's owner hears it on. No handler here calls a provider — the
//! ones that stand in for Veo count what they would have paid for.

#[path = "../common/mod.rs"]
mod common;

mod claim;
mod live;
mod recover;
mod worker;

use std::time::Duration;

use scorsese_server::accounts::users;
use scorsese_server::db::{self, UserId};
use scorsese_server::jobs::{JobView, Kind, Queue, Registry, State, store, work};
use serde_json::Value;
use sqlx::postgres::PgPool;
use tokio::sync::watch;
use tokio::task::JoinHandle;

/// The kind these tests run: two at a time.
const ECHO: Kind = Kind {
    name: "echo",
    limit: 2,
};

/// A second kind, to show a claim keeps to the kinds it is asked for.
const OTHER: Kind = Kind {
    name: "other",
    limit: 1,
};

/// A new account for `email`.
async fn account(pool: &PgPool, email: &str) -> UserId {
    users::create(pool, email, "password one")
        .await
        .expect("a fresh database takes a new account")
}

/// The pool the server answers from: nothing readable outside a scope.
async fn members(pool: &PgPool) -> PgPool {
    db::member_pool(pool)
        .await
        .expect("the member pool connects")
}

/// Enqueue a job of `kind` for `user`, committed.
async fn enqueue(members: &PgPool, user: UserId, kind: Kind, payload: Value) -> JobView {
    let mut tx = db::scoped(members, user).await.expect("a scope opens");
    let job = store::enqueue(&mut tx, kind, &payload)
        .await
        .expect("a job is enqueued");
    tx.commit().await.expect("the enqueue commits");
    job
}

/// A worker running `registry` until the returned sender says stop.
fn start_worker(
    members: &PgPool,
    registry: Registry,
    queue: &Queue,
) -> (watch::Sender<bool>, JoinHandle<()>) {
    let (stop, stopping) = watch::channel(false);
    let worker = tokio::spawn(work(members.clone(), registry, queue.clone(), stopping));
    (stop, worker)
}

/// Stop a worker and wait for it to have stopped.
async fn stop_worker((stop, worker): (watch::Sender<bool>, JoinHandle<()>)) {
    stop.send_replace(true);
    tokio::time::timeout(Duration::from_secs(10), worker)
        .await
        .expect("the worker stops promptly")
        .expect("the worker did not panic");
}

/// Job `id` as `user` sees it, once it is in `state`.
async fn when(members: &PgPool, user: UserId, id: i64, state: State) -> JobView {
    for _ in 0..200 {
        let job = store::get(members, user, id)
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
