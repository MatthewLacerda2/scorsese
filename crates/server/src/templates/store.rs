//! The queries: each in a [`db::scoped`] transaction with no owner filter —
//! the row-level policy is the filter — except the startup migration, which is
//! the operator's build acting on every account at once.

use scorsese_core::{Project, SCHEMA_VERSION, migrate};
use sqlx::postgres::PgPool;

use super::{Stored, Summary, TemplateError};
use crate::db::{self, Tx, UserId};
use crate::projects::media::hashes;

type Row = (i64, i64, String);

/// A row read back into a template.
fn stored((id, updated_at, json): Row) -> Result<Stored, TemplateError> {
    let document =
        Project::from_json(&json).map_err(|source| TemplateError::Unreadable { id, source })?;
    Ok(Stored {
        summary: Summary::of(id, updated_at, &document),
        document,
    })
}

/// `user`'s templates, by name.
pub async fn list(pool: &PgPool, user: UserId) -> Result<Vec<Summary>, TemplateError> {
    let mut tx = db::scoped(pool, user).await?;
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, extract(epoch FROM updated_at)::bigint, document::text FROM templates
         ORDER BY lower(name), id",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    rows.into_iter()
        .map(|row| stored(row).map(|template| template.summary))
        .collect()
}

/// `user`'s template `id`.
pub async fn open(pool: &PgPool, user: UserId, id: i64) -> Result<Stored, TemplateError> {
    let mut tx = db::scoped(pool, user).await?;
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, extract(epoch FROM updated_at)::bigint, document::text FROM templates
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    stored(row.ok_or(TemplateError::NotFound)?)
}

/// Keep `template` as one of `user`'s, under its own name — over the template
/// already called that when `replace`, refused with
/// [`TemplateError::NameTaken`] otherwise.
pub async fn save(
    pool: &PgPool,
    user: UserId,
    template: &Project,
    replace: bool,
) -> Result<Summary, TemplateError> {
    if template.name.trim().is_empty() {
        return Err(TemplateError::Unnamed);
    }
    let json = serde_json::to_string(template)?;
    let mut tx = db::scoped(pool, user).await?;
    let taken: Option<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM templates WHERE lower(name) = lower($1) FOR UPDATE")
            .bind(&template.name)
            .fetch_optional(&mut *tx)
            .await?;
    let (id, updated_at): (i64, i64) = match taken {
        Some((id, name)) if !replace => return Err(TemplateError::NameTaken { id, name }),
        Some((id, _)) => {
            sqlx::query_as(
                "UPDATE templates SET document = $2::jsonb, updated_at = now() WHERE id = $1
                 RETURNING id, extract(epoch FROM updated_at)::bigint",
            )
            .bind(id)
            .bind(json)
            .fetch_one(&mut *tx)
            .await?
        }
        None => {
            sqlx::query_as(
                "INSERT INTO templates (user_id, document) VALUES (member_id(), $1::jsonb)
                 RETURNING id, extract(epoch FROM updated_at)::bigint",
            )
            .bind(json)
            .fetch_one(&mut *tx)
            .await?
        }
    };
    record(&mut tx, id, template).await?;
    tx.commit().await?;
    Ok(Summary::of(id, updated_at, template))
}

/// Delete `user`'s template `id`. `false` when they have none by that id.
pub async fn delete(pool: &PgPool, user: UserId, id: i64) -> Result<bool, TemplateError> {
    let mut tx = db::scoped(pool, user).await?;
    let deleted = sqlx::query("DELETE FROM templates WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;
    Ok(deleted == 1)
}

/// Rewrite template `id`'s `template_assets` from its document — refused,
/// naming the assets, when one names a file its owner's library lacks.
async fn record(tx: &mut Tx, id: i64, template: &Project) -> Result<(), TemplateError> {
    sqlx::query("DELETE FROM template_assets WHERE template_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    let hashes: Vec<String> = hashes(template).into_iter().map(str::to_owned).collect();
    let unknown: Vec<String> = sqlx::query_scalar(
        "SELECT h FROM unnest($2::text[]) AS h WHERE NOT EXISTS (
             SELECT 1 FROM templates t JOIN library_items l ON l.user_id = t.user_id
             WHERE t.id = $1 AND l.sha256 = h)",
    )
    .bind(id)
    .bind(&hashes)
    .fetch_all(&mut **tx)
    .await?;
    if !unknown.is_empty() {
        let assets = template
            .assets
            .iter()
            .filter(|asset| asset.sha256.as_ref().is_some_and(|h| unknown.contains(h)))
            .map(|asset| asset.id.to_string())
            .collect();
        return Err(TemplateError::UnknownFiles(assets));
    }
    sqlx::query(
        "INSERT INTO template_assets (template_id, user_id, sha256)
         SELECT t.id, t.user_id, h FROM templates t, unnest($2::text[]) AS h WHERE t.id = $1",
    )
    .bind(id)
    .bind(hashes)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Bring every stored template written by another build to this one's
/// `schema_version`, and say how many there were — beside
/// [`crate::projects::migrate_stored`], and all or nothing for the same reason.
pub async fn migrate_stored(pool: &PgPool) -> Result<usize, TemplateError> {
    let mut tx = db::privileged(pool).await?;
    let behind: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, document::text FROM templates
         WHERE document -> 'schema_version' IS DISTINCT FROM to_jsonb($1::bigint)
         ORDER BY id FOR UPDATE",
    )
    .bind(i64::from(SCHEMA_VERSION))
    .fetch_all(&mut *tx)
    .await?;
    for (id, json) in &behind {
        let (template, _) =
            migrate::parse(json).map_err(|source| TemplateError::Migrate { id: *id, source })?;
        sqlx::query("UPDATE templates SET document = $2::jsonb WHERE id = $1")
            .bind(id)
            .bind(serde_json::to_string(&template)?)
            .execute(&mut *tx)
            .await?;
        record(&mut tx, *id, &template).await?;
    }
    tx.commit().await?;
    Ok(behind.len())
}
