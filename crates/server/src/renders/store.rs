//! The `renders` table's queries a user's own requests make: every one in a
//! scoped transaction, none with an owner filter — the policy is the filter
//! (`db::scope`). The cross-user ones are eviction's, in [`super::evict`].

use sqlx::postgres::PgPool;

use super::{RenderView, Settings};
use crate::db::{self, Tx, UserId};
use crate::jobs::JobView;

/// The columns a [`RenderView`] is read from. A macro so each query built
/// from it is still one literal, which is what sqlx accepts.
macro_rules! view {
    () => {
        "id, project_id AS project, key, settings, size,
         extract(epoch FROM created_at)::bigint AS created_at,
         extract(epoch FROM last_used_at)::bigint AS last_used_at"
    };
}

/// The render of `project` kept under `key`, stamped as used now, and where
/// its file is — or `None` when there is none.
pub async fn take(
    tx: &mut Tx,
    project: i64,
    key: &str,
) -> Result<Option<(RenderView, String)>, sqlx::Error> {
    let row: Option<Taken> = sqlx::query_as(concat!(
        "UPDATE renders SET last_used_at = now() WHERE project_id = $1 AND key = $2
         RETURNING path, ",
        view!()
    ))
    .bind(project)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(|row| (row.view, row.path)))
}

/// A row, and where its file is.
#[derive(sqlx::FromRow)]
struct Taken {
    path: String,
    #[sqlx(flatten)]
    view: RenderView,
}

/// Forget render `id`: its file is gone, so the row promises nothing.
pub async fn forget(tx: &mut Tx, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM renders WHERE id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await
        .map(drop)
}

/// The render job already waiting or running for `project` under `key`, so a
/// second request joins it instead of queueing the same work twice.
pub async fn pending(tx: &mut Tx, project: i64, key: &str) -> Result<Option<JobView>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, kind, state, attempts, result, error,
                extract(epoch FROM created_at)::bigint AS created_at,
                extract(epoch FROM started_at)::bigint AS started_at,
                extract(epoch FROM finished_at)::bigint AS finished_at,
                extract(epoch FROM interrupted_at)::bigint AS interrupted_at
         FROM jobs WHERE kind = 'render' AND state IN ('waiting', 'running')
           AND payload ->> 'project' = $1::text AND payload ->> 'key' = $2
         ORDER BY id LIMIT 1",
    )
    .bind(project)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
}

/// Record a finished render of `project`, as the user `tx` is scoped to.
///
/// A second render of the same key — two requests that raced past
/// [`pending`] — lands on the first row and stamps it used.
pub async fn insert(
    tx: &mut Tx,
    project: i64,
    key: &str,
    settings: &Settings,
    path: &str,
    size: i64,
) -> Result<RenderView, sqlx::Error> {
    sqlx::query_as(concat!(
        "INSERT INTO renders (user_id, project_id, key, settings, path, size)
         VALUES (member_id(), $1, $2, $3, $4, $5)
         ON CONFLICT (project_id, key) DO UPDATE SET last_used_at = now()
         RETURNING ",
        view!()
    ))
    .bind(project)
    .bind(key)
    .bind(sqlx::types::Json(settings))
    .bind(path)
    .bind(size)
    .fetch_one(&mut **tx)
    .await
}

/// `user`'s renders of `project`, most recently used first.
pub async fn list(
    pool: &PgPool,
    user: UserId,
    project: i64,
) -> Result<Vec<RenderView>, sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    let renders = sqlx::query_as(concat!(
        "SELECT ",
        view!(),
        " FROM renders WHERE project_id = $1 ORDER BY last_used_at DESC, id DESC"
    ))
    .bind(project)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(renders)
}

/// A render being downloaded: what it is, where its file is, and the name to
/// offer it under.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Download {
    /// Where the file is, relative to the cache root.
    pub path: String,
    /// What shape of file it is.
    #[sqlx(json)]
    pub settings: Settings,
    /// Its project's name.
    pub name: String,
}

/// `user`'s render `id`, **stamped as used and committed** before anything
/// opens its file — the order eviction relies on (see [`super`]).
pub async fn download(
    pool: &PgPool,
    user: UserId,
    id: i64,
) -> Result<Option<Download>, sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    let found = sqlx::query_as(
        "UPDATE renders r SET last_used_at = now() FROM projects p
         WHERE r.id = $1 AND p.id = r.project_id
         RETURNING r.path, r.settings, p.name",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(found)
}
