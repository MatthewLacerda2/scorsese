//! The `jobs` table's queries.
//!
//! Two are privileged, because they are cross-user by nature: [`claim`] (which
//! job next, whoever's it is) and [`recover`] (every job a dead process left
//! running), plus [`interrupted`], the operator's view of a crash. Everything
//! else runs scoped as the job's owner — see the module doc of
//! [`jobs`](super).

use serde_json::Value;
use sqlx::postgres::PgPool;

use super::{Job, JobView, Kind, MAX_ATTEMPTS, Outcome};
use crate::db::{self, Tx, UserId};

/// The columns a [`JobView`] is read from. A macro rather than a constant so
/// each query is still one `&'static str` — sqlx refuses a `format!`ed one,
/// which is the point: a query assembled at run time is where injection lives.
macro_rules! view {
    () => {
        "id, kind, state, attempts, result, error,
         extract(epoch FROM created_at)::bigint AS created_at,
         extract(epoch FROM started_at)::bigint AS started_at,
         extract(epoch FROM finished_at)::bigint AS finished_at,
         extract(epoch FROM interrupted_at)::bigint AS interrupted_at"
    };
}

/// Put a job of `kind` in line for the user `tx` is scoped to.
///
/// Takes the caller's transaction so a job and what it belongs to — a render
/// and its project, say — are committed together or not at all. Once it is
/// committed, [`Queue::announce`](super::Queue::announce) tells the worker
/// and the owner's browser; without that the worker still finds it on its
/// next look, a second or so later.
pub async fn enqueue(tx: &mut Tx, kind: Kind, payload: &Value) -> Result<JobView, sqlx::Error> {
    sqlx::query_as(concat!(
        "INSERT INTO jobs (user_id, kind, payload) VALUES (member_id(), $1, $2) RETURNING ",
        view!()
    ))
    .bind(kind.name)
    .bind(payload)
    .fetch_one(&mut **tx)
    .await
}

/// `user`'s jobs, newest first — the last hundred.
pub async fn list(pool: &PgPool, user: UserId) -> Result<Vec<JobView>, sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    let jobs = sqlx::query_as(concat!(
        "SELECT ",
        view!(),
        " FROM jobs ORDER BY id DESC LIMIT 100"
    ))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(jobs)
}

/// `user`'s job `id`, or `None` — including when it is somebody else's.
pub async fn get(pool: &PgPool, user: UserId, id: i64) -> Result<Option<JobView>, sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    let job = sqlx::query_as(concat!("SELECT ", view!(), " FROM jobs WHERE id = $1"))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(job)
}

/// Take the next waiting job of one of `kinds`, whoever's it is, and mark it
/// running.
///
/// `SKIP LOCKED`, so a job another claim is taking is passed over rather than
/// waited on or taken twice. Fair between users: the owner with the fewest
/// jobs running goes first, and the oldest job among equals.
pub async fn claim(pool: &PgPool, kinds: &[&str]) -> Result<Option<(Job, JobView)>, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let row: Option<ClaimRow> = sqlx::query_as(concat!(
        "UPDATE jobs SET state = 'running', attempts = attempts + 1,
                started_at = now(), finished_at = NULL, error = NULL
         WHERE id = (
             SELECT id FROM jobs j WHERE state = 'waiting' AND kind = ANY($1)
             ORDER BY (SELECT count(*) FROM jobs r
                       WHERE r.user_id = j.user_id AND r.state = 'running'), id
             LIMIT 1 FOR UPDATE SKIP LOCKED)
         RETURNING user_id, payload, ticket, ",
        view!()
    ))
    .bind(kinds)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row.map(|row| {
        let job = Job {
            id: row.view.id,
            user: UserId::from_row(row.user_id),
            kind: row.view.kind.clone(),
            payload: row.payload,
            attempts: row.view.attempts,
            ticket: row.ticket,
        };
        (job, row.view)
    }))
}

/// A claimed row: the job, and what its owner sees of it.
#[derive(sqlx::FromRow)]
struct ClaimRow {
    user_id: i64,
    payload: Value,
    ticket: Option<String>,
    #[sqlx(flatten)]
    view: JobView,
}

/// Commit `ticket` to job `id`'s row, as its owner.
pub(super) async fn keep_ticket(
    pool: &PgPool,
    user: UserId,
    id: i64,
    ticket: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    sqlx::query("UPDATE jobs SET ticket = $2, ticket_at = now() WHERE id = $1")
        .bind(id)
        .bind(ticket)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// Record how `job`'s run ended, as its owner.
pub(super) async fn finish(
    pool: &PgPool,
    job: &Job,
    outcome: &Outcome,
) -> Result<JobView, sqlx::Error> {
    let (state, result, error) = match outcome {
        Outcome::Done(result) => ("done", Some(result), None),
        Outcome::Failed(why) => ("failed", None, Some(why)),
        Outcome::Stuck(why) => ("stuck", None, Some(why)),
    };
    let mut tx = db::scoped(pool, job.user).await?;
    let view = sqlx::query_as(concat!(
        "UPDATE jobs SET state = $2, result = $3, error = $4, finished_at = now()
         WHERE id = $1 RETURNING ",
        view!()
    ))
    .bind(job.id)
    .bind(state)
    .bind(result)
    .bind(error)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(view)
}

/// Every job a dead process left running, put back in line — or given up on
/// after [`MAX_ATTEMPTS`]. Each is stamped `interrupted_at`.
///
/// Only correct while no other worker is running, which the worker's advisory
/// lock guarantees before it calls this.
pub async fn recover(pool: &PgPool) -> Result<Vec<(UserId, JobView)>, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let rows: Vec<Recovered> = sqlx::query_as(concat!(
        "UPDATE jobs SET interrupted_at = now(),
            state = CASE WHEN attempts < $1 THEN 'waiting'
                         WHEN ticket IS NOT NULL THEN 'stuck' ELSE 'failed' END,
            error = CASE WHEN attempts < $1 THEN NULL
                         ELSE 'interrupted ' || attempts || ' times, so not tried again' END,
            finished_at = CASE WHEN attempts < $1 THEN NULL ELSE now() END
         WHERE state = 'running' RETURNING user_id, ",
        view!()
    ))
    .bind(MAX_ATTEMPTS)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|row| (UserId::from_row(row.user_id), row.view))
        .collect())
}

/// A recovered row: whose, and what they see.
#[derive(sqlx::FromRow)]
struct Recovered {
    user_id: i64,
    #[sqlx(flatten)]
    view: JobView,
}

/// A job a crash cut off, as the operator reads it.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Interruption {
    /// The job.
    pub id: i64,
    /// Whose: the account's email.
    pub email: String,
    /// Its kind.
    pub kind: String,
    /// Where it is now.
    pub state: String,
    /// When it was last cut off, in UTC to the second.
    pub interrupted_at: String,
}

/// The last two hundred jobs a crash or restart cut off, newest first, with
/// whose they are. Privileged: the operator's view, across every user.
pub async fn interrupted(pool: &PgPool) -> Result<Vec<Interruption>, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let rows = sqlx::query_as(
        "SELECT j.id, u.email, j.kind, j.state,
                to_char(j.interrupted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS \"UTC\"')
                    AS interrupted_at
         FROM jobs j JOIN users u ON u.id = j.user_id
         WHERE j.interrupted_at IS NOT NULL
         ORDER BY j.interrupted_at DESC, j.id LIMIT 200",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows)
}
