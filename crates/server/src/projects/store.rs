//! The queries: every one in a [`db::scoped`] transaction, and none with an
//! owner filter — the row-level policy is the filter (`db::scope`).

use scorsese_core::Project;
use sqlx::postgres::PgPool;

use super::{ProjectError, Stored, Summary, media};
use crate::db::{self, Tx, UserId};

/// The columns a [`Summary`] is read from, in its order. A macro so that the
/// queries built from it are still literals, which is what sqlx accepts.
macro_rules! summary_columns {
    () => {
        "id, name, revision, extract(epoch FROM created_at)::bigint, \
         extract(epoch FROM updated_at)::bigint"
    };
}

type SummaryRow = (i64, String, i64, i64, i64);

fn summary((id, name, revision, created_at, updated_at): SummaryRow) -> Summary {
    Summary {
        id,
        name,
        revision,
        created_at,
        updated_at,
    }
}

/// `user`'s projects, most recently written first.
pub async fn list(pool: &PgPool, user: UserId) -> Result<Vec<Summary>, ProjectError> {
    let mut tx = db::scoped(pool, user).await?;
    let rows: Vec<SummaryRow> = sqlx::query_as(concat!(
        "SELECT ",
        summary_columns!(),
        " FROM projects ORDER BY updated_at DESC, id DESC"
    ))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows.into_iter().map(summary).collect())
}

/// Store `project` as a new project of `user`'s.
pub async fn create(
    pool: &PgPool,
    user: UserId,
    project: &Project,
) -> Result<Summary, ProjectError> {
    let json = serde_json::to_string(project)?;
    let mut tx = db::scoped(pool, user).await?;
    let row: SummaryRow = sqlx::query_as(concat!(
        "INSERT INTO projects (user_id, document) VALUES (member_id(), $1::jsonb) RETURNING ",
        summary_columns!()
    ))
    .bind(json)
    .fetch_one(&mut *tx)
    .await?;
    media::record(&mut tx, row.0, project).await?;
    tx.commit().await?;
    Ok(summary(row))
}

/// `user`'s project `id`, with the revision a save of it has to name.
pub async fn open(pool: &PgPool, user: UserId, id: i64) -> Result<Stored, ProjectError> {
    let mut tx = db::scoped(pool, user).await?;
    let stored = read(&mut tx, id, false).await?;
    tx.commit().await?;
    Ok(stored)
}

/// Replace project `id`'s document with `project`, provided `revision` is
/// still the current one. The new revision on success.
///
/// [`ProjectError::Conflict`] when somebody wrote in between — nothing was
/// written, and the caller re-reads and redoes its edit.
pub async fn save(
    pool: &PgPool,
    user: UserId,
    id: i64,
    revision: i64,
    project: &Project,
) -> Result<i64, ProjectError> {
    let mut tx = db::scoped(pool, user).await?;
    let Some(written) = write(&mut tx, id, revision, project).await? else {
        let current: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM projects WHERE id = $1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        return Err(
            current.map_or(ProjectError::NotFound, |current| ProjectError::Conflict {
                current,
            }),
        );
    };
    tx.commit().await?;
    Ok(written)
}

/// Read, `change` and write project `id` in one transaction, with the row
/// locked throughout — what `change` returns, and the new revision.
///
/// Only for a change that takes no time, like a rename: the lock holds every
/// other writer until this commits. An edit that thinks — an assistant's —
/// reads with [`open`] and writes with [`save`] instead.
pub async fn edit<T>(
    pool: &PgPool,
    user: UserId,
    id: i64,
    change: impl FnOnce(&mut Project) -> T,
) -> Result<(T, i64), ProjectError> {
    let mut tx = db::scoped(pool, user).await?;
    let Stored {
        summary,
        document: mut project,
    } = read(&mut tx, id, true).await?;
    let answer = change(&mut project);
    let written = write(&mut tx, id, summary.revision, &project)
        .await?
        .ok_or(ProjectError::NotFound)?;
    tx.commit().await?;
    Ok((answer, written))
}

/// Delete `user`'s project `id`. `false` when they have none by that id.
pub async fn delete(pool: &PgPool, user: UserId, id: i64) -> Result<bool, ProjectError> {
    let mut tx = db::scoped(pool, user).await?;
    let deleted = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;
    Ok(deleted == 1)
}

/// Write `project` over row `id` if it is at `revision`, and re-derive which
/// files it uses. `None` when no row matched — gone, or moved on.
///
/// The one way a document reaches the table after it is created, so the
/// revision rule and `project_assets` cannot be skipped by any caller.
pub(super) async fn write(
    tx: &mut Tx,
    id: i64,
    revision: i64,
    project: &Project,
) -> Result<Option<i64>, ProjectError> {
    let json = serde_json::to_string(project)?;
    let written: Option<i64> = sqlx::query_scalar(
        "UPDATE projects SET document = $3::jsonb, revision = revision + 1, updated_at = now()
         WHERE id = $1 AND revision = $2 RETURNING revision",
    )
    .bind(id)
    .bind(revision)
    .bind(json)
    .fetch_optional(&mut **tx)
    .await?;
    if written.is_some() {
        media::record(tx, id, project).await?;
    }
    Ok(written)
}

/// Row `id` and its document, in one statement so the revision is the
/// document's own — and, when `lock`, locked until the transaction ends.
async fn read(tx: &mut Tx, id: i64, lock: bool) -> Result<Stored, ProjectError> {
    macro_rules! select {
        () => {
            concat!(
                "SELECT ",
                summary_columns!(),
                ", document::text FROM projects WHERE id = $1"
            )
        };
    }
    let sql = if lock {
        concat!(select!(), " FOR UPDATE")
    } else {
        select!()
    };
    let (id, name, revision, created_at, updated_at, json): (i64, String, i64, i64, i64, String) =
        sqlx::query_as(sql)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(ProjectError::NotFound)?;
    let document =
        Project::from_json(&json).map_err(|source| ProjectError::Unreadable { id, source })?;
    Ok(Stored {
        summary: summary((id, name, revision, created_at, updated_at)),
        document,
    })
}
