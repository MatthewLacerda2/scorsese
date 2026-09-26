//! Credits (`src/credits`): the ledger, paid generations, the monthly fee,
//! the history a user reads, and the operator's commands. No provider is
//! called: a generation here is recorded, reserved for and settled as a job
//! handler would, with the provider's answer made up.

#[path = "../common/mod.rs"]
mod common;

mod append_only;
mod commands;
mod fees;
mod filters;
mod history;
mod projects;
mod routes;
mod spending;
mod tool;

use scorsese_server::accounts::users;
use scorsese_server::credits::generations::{self, Answer, Paid, Request, Shot};
use scorsese_server::credits::{CreditError, ledger};
use scorsese_server::db::{self, UserId};
use sqlx::postgres::PgPool;

/// A new account for `email`.
async fn account(pool: &PgPool, email: &str) -> UserId {
    users::create(pool, email, "password one")
        .await
        .expect("a fresh database takes a new account")
}

/// Credit `user` with `dollars`, as a top-up at five reais to the dollar.
async fn fund(pool: &PgPool, user: UserId, dollars: i64) {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    let credited = ledger::top_up(&mut tx, dollars * 500, 50_000)
        .await
        .expect("a top-up is recorded");
    assert_eq!(credited, dollars * 1_000_000);
    tx.commit().await.expect("the top-up commits");
}

/// `user`'s balance, in micro-dollars.
async fn balance(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    ledger::balance(&mut tx).await.expect("the balance reads")
}

/// An eight-second Fast shot at 1080p for `project`, quoted at 96 cents.
fn shot(project: Option<i64>) -> Request<'static> {
    Request::Shot(Shot {
        project,
        tool_call: None,
        job: None,
        model: "fast",
        resolution: "1080p",
        seconds: 8,
        aspect: "16:9",
        prompt: "a lighthouse at dusk",
        brief_hash: "abc123",
        estimated_cents: 96,
    })
}

/// What a 96-cent shot costs the user: 96 cents plus 10%.
const SHOT_PRICE: i64 = 1_056_000;

/// Start a generation for `user`, committed when it was not refused.
async fn start(pool: &PgPool, user: UserId, request: &Request<'_>) -> Result<Paid, CreditError> {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    let paid = generations::start(&mut tx, request).await?;
    tx.commit().await.expect("the start commits");
    Ok(paid)
}

/// Settle a generation for `user` with the provider's `answer`, committed.
async fn finish(
    pool: &PgPool,
    user: UserId,
    paid: Paid,
    answer: &Answer,
) -> Result<(), CreditError> {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    generations::finish(&mut tx, paid, answer).await?;
    tx.commit().await.expect("the finish commits");
    Ok(())
}
