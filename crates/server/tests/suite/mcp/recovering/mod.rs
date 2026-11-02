//! A paid generation the server stopped in the middle of (#1645): never sent
//! to the vendor twice, charged when its output was kept, released when it
//! was not — and never left holding the member's credit.

use std::sync::atomic::Ordering;
use std::time::Duration;

use scorsese_server::credits::{generations, ledger};
use scorsese_server::db::{self, UserId};
use scorsese_server::jobs::elevenlabs::ElevenLabs;
use scorsese_server::jobs::{MAX_ATTEMPTS, store};
use sqlx::postgres::PgPool;

mod charging;

use super::paying::{fund, narrated, quote_and_confirm};
use super::vendors::{Mock, serve_idle, settled, work};
use super::{member, stored};

/// What ana is funded with, in micro-dollars.
pub(super) const FUNDED: i64 = 10_000_000;

#[sqlx::test]
async fn a_job_given_up_on_lets_go_of_its_reservation(pool: PgPool) {
    let (address, state, _files) = serve_idle(&pool).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, narrated()).await;
    fund(&pool, ana, 10).await;
    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    let job = only_job(&pool).await;
    assert!(open(&pool, ana, job).await, "the yes reserved");

    // Cut off as often as a job may be.
    sqlx::query("UPDATE jobs SET state = 'running', attempts = $1")
        .bind(MAX_ATTEMPTS)
        .execute(&pool)
        .await
        .expect("the database answers");
    store::recover(&state.pool)
        .await
        .expect("the database answers");

    let view = store::get(&state.pool, ana, job)
        .await
        .expect("the database answers")
        .expect("the database answers");
    assert_eq!(format!("{:?}", view.state), "Failed");
    assert!(!open(&pool, ana, job).await, "released with the give-up");
    assert_eq!(balance(&pool, ana).await, FUNDED, "nothing was charged");
}

#[sqlx::test]
async fn a_line_stopped_mid_call_is_never_spoken_again_and_ends_released(pool: PgPool) {
    let mock = Mock::default();
    mock.hold.store(true, Ordering::SeqCst);
    let (address, state, files) = serve_idle(&pool).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, narrated()).await;
    fund(&pool, ana, 10).await;
    let first = work(&pool, &state, &files, &mock, ElevenLabs::default());
    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    until("the line is asked for", || mock.spoken() == 1).await;

    // The server stops with the call out; ElevenLabs answers into nothing.
    first.send(true).expect("the worker listens");
    tokio::time::sleep(Duration::from_millis(200)).await;
    mock.hold.store(false, Ordering::SeqCst);
    let _second = work(&pool, &state, &files, &mock, ElevenLabs::default());

    assert_eq!(settled(&state, ana).await, vec!["failed"]);
    assert_eq!(mock.spoken(), 1, "never sent twice");
    let job = only_job(&pool).await;
    assert!(!open(&pool, ana, job).await, "settled");
    assert_eq!(balance(&pool, ana).await, FUNDED, "and released, free");
}

/// Wait until `done` holds.
pub(super) async fn until(what: &str, done: impl Fn() -> bool) {
    for _ in 0..400 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("{what} never happened");
}

/// The one spoken-line job there is.
pub(super) async fn only_job(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT id FROM jobs WHERE kind = 'spoken_line'")
        .fetch_one(pool)
        .await
        .expect("the database answers")
}

/// Whether job `job` still holds a reservation of `user`'s.
pub(super) async fn open(pool: &PgPool, user: UserId, job: i64) -> bool {
    let mut tx = db::scoped(pool, user).await.expect("the database answers");
    let paid = generations::for_job(&mut tx, job)
        .await
        .expect("the database answers");
    tx.commit().await.expect("the database answers");
    paid.is_some()
}

/// `user`'s paid balance.
pub(super) async fn balance(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.expect("the database answers");
    let balance = ledger::balance(&mut tx)
        .await
        .expect("the database answers");
    tx.commit().await.expect("the database answers");
    balance
}
