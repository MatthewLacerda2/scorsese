//! Letting go of the database before `#[sqlx::test]` drops it (#1126).
//!
//! A test's last act is sqlx's: close the test's pool, then `DROP DATABASE`.
//! Postgres refuses that drop while any other session is still on the
//! database, after waiting five seconds for it to leave, and sqlx only
//! prints the failure. A member pool still held by a spawned server or
//! worker is exactly such a session, so a test that left one behind paid
//! those five seconds and leaked its database.
//!
//! sqlx's closing of the test pool is the signal: whatever is registered
//! here hears it, aborts its tasks and closes its member pool, which takes
//! milliseconds, inside the window Postgres waits. Nothing is held by the
//! test, so nothing can be dropped too early (a guard bound to `_` would
//! be), and a test that returns early lets go all the same.

use sqlx::postgres::PgPool;
use tokio::task::AbortHandle;

/// When the test's own `pool` closes, abort `tasks` and close `members`.
pub(crate) fn release_at_teardown(
    pool: &PgPool,
    members: &PgPool,
    tasks: impl IntoIterator<Item = AbortHandle>,
) {
    let closing = pool.close_event();
    let members = members.clone();
    let tasks: Vec<AbortHandle> = tasks.into_iter().collect();
    tokio::spawn(async move {
        closing.await;
        tasks.iter().for_each(AbortHandle::abort);
        members.close().await;
    });
}

/// The member pool on `pool`, closed when the test ends.
pub(crate) async fn members(pool: &PgPool) -> PgPool {
    let members = scorsese_server::db::member_pool(pool)
        .await
        .expect("the member pool connects");
    release_at_teardown(pool, &members, []);
    members
}
