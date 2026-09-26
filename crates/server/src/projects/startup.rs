//! Carrying every stored document forward when the server starts on a new build.

use scorsese_core::{SCHEMA_VERSION, migrate};
use sqlx::postgres::PgPool;

use super::{ProjectError, store};
use crate::db;

/// Bring every stored project written by another build to this one's
/// `schema_version`, and say how many there were.
///
/// Runs before the server accepts a connection (`crate::start`), so no
/// request ever reads a document this build does not understand. **All or
/// nothing**: one transaction, and one document that cannot be carried
/// forward — a newer build's, or one a step refuses — stops the server from
/// starting with that project named, rather than leaving some projects
/// migrated and others not. A newer build's documents are the case to
/// expect: it means an older binary was deployed over a newer one, and the
/// way back is the dump taken before the update (`docs/web.md`, *Updating*).
///
/// Privileged, because a migration is cross-user by nature: it is the
/// operator's build acting on every account at once. A migrated document's
/// revision moves on like any write, so a client holding the old one is
/// refused rather than overwriting the migration.
pub async fn migrate_stored(pool: &PgPool) -> Result<usize, ProjectError> {
    let mut tx = db::privileged(pool).await?;
    let behind: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT id, revision, document::text FROM projects
         WHERE document -> 'schema_version' IS DISTINCT FROM to_jsonb($1::bigint)
         ORDER BY id FOR UPDATE",
    )
    .bind(i64::from(SCHEMA_VERSION))
    .fetch_all(&mut *tx)
    .await?;
    for (id, revision, json) in &behind {
        let (project, _) =
            migrate::parse(json).map_err(|source| ProjectError::Migrate { id: *id, source })?;
        store::write(&mut tx, *id, *revision, &project).await?;
    }
    tx.commit().await?;
    Ok(behind.len())
}
