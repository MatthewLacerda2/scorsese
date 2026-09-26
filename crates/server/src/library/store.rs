//! Reading and changing a user's items. Every query runs in a
//! [`db::scoped`] transaction with no owner filter: the policy is the filter.

use serde::Deserialize;

use super::{Item, Kind, Library, LibraryError, Summary, UsedBy};
use crate::db::{self, Tx, UserId};

/// The columns an [`Item`] is read from, in [`ItemRow`]'s order. A macro so a
/// query built from it is still one literal.
macro_rules! item_columns {
    () => {
        "id, name, kind, sha256, extension, size_bytes, media::text, description, brief_hash, \
         extract(epoch FROM created_at)::bigint, extract(epoch FROM last_used_at)::bigint"
    };
}
pub(super) use item_columns;

/// An item as a row: [`item_columns!`] in order.
pub(super) type ItemRow = (
    i64,
    String,
    String,
    String,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    i64,
    Option<i64>,
);

/// The item a row describes.
pub(super) fn item(row: ItemRow) -> Result<Item, LibraryError> {
    let (id, name, kind, sha256, extension, size_bytes, media, description, brief_hash, created_at, last_used_at) =
        row;
    let kind = Kind::try_from(kind).map_err(LibraryError::Invalid)?;
    let media = serde_json::from_str(&media)
        .map_err(|error| LibraryError::Invalid(format!("item {id}'s media: {error}")))?;
    Ok(Item {
        id,
        name,
        kind,
        sha256,
        extension,
        size_bytes,
        media,
        description,
        brief_hash,
        created_at,
        last_used_at,
    })
}

/// Which items a list shows. Every field narrows it; none set is everything.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Filter {
    /// Only this kind.
    pub kind: Option<Kind>,
    /// Only names containing this, ignoring case.
    pub search: Option<String>,
    /// Only the item with these bytes — what the browser asks before
    /// uploading, so a duplicate never crosses the network.
    pub sha256: Option<String>,
}

/// What `PATCH /api/library/{id}` may change. Absent fields stay as they are.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Change {
    /// A new name.
    pub name: Option<String>,
    /// A new description; an empty one removes it.
    pub description: Option<String>,
}

impl Library {
    /// `user`'s items, newest first, narrowed by `filter`.
    pub async fn list(&self, user: UserId, filter: &Filter) -> Result<Vec<Summary>, LibraryError> {
        let search = filter
            .search
            .as_deref()
            .map(str::trim)
            .filter(|search| !search.is_empty())
            .map(|search| format!("%{}%", escape_like(search)));
        let mut tx = db::scoped(&self.pool, user).await?;
        let items = sqlx::query_as(
            "SELECT id, name, kind, size_bytes FROM library_items
             WHERE ($1::text IS NULL OR kind = $1)
               AND ($2::text IS NULL OR name ILIKE $2)
               AND ($3::text IS NULL OR sha256 = $3)
             ORDER BY id DESC",
        )
        .bind(filter.kind.map(Kind::as_str))
        .bind(search)
        .bind(filter.sha256.as_deref())
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(items)
    }

    /// `user`'s item `id`.
    pub async fn get(&self, user: UserId, id: i64) -> Result<Item, LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        let item = read(&mut tx, id).await?;
        tx.commit().await?;
        Ok(item)
    }

    /// The projects of `user`'s that use item `id`, by name.
    pub async fn used_by(&self, user: UserId, id: i64) -> Result<Vec<UsedBy>, LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        let projects = projects_using(&mut tx, id).await?;
        tx.commit().await?;
        Ok(projects)
    }

    /// Rename or describe item `id`.
    pub async fn update(&self, user: UserId, id: i64, change: &Change) -> Result<Item, LibraryError> {
        let name = match change.name.as_deref().map(str::trim) {
            Some("") => return Err(LibraryError::Invalid("a file needs a name".to_owned())),
            name => name,
        };
        let description = change.description.as_deref().map(str::trim);
        let mut tx = db::scoped(&self.pool, user).await?;
        let changed = sqlx::query(
            "UPDATE library_items SET name = coalesce($2, name),
                description = CASE WHEN $3::text IS NULL THEN description
                                   ELSE nullif($3, '') END
             WHERE id = $1",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if changed == 0 {
            return Err(LibraryError::NotFound);
        }
        let item = read(&mut tx, id).await?;
        tx.commit().await?;
        Ok(item)
    }

    /// Delete item `id` and its file — unless a project uses it, which is
    /// refused with the projects' names (#396 decides nothing here: a project
    /// keeps its files until it lets go of them).
    ///
    /// The row goes first and the file after the commit, so a failure between
    /// the two leaves a file nobody names rather than a name with no file.
    pub async fn delete(&self, user: UserId, id: i64) -> Result<(), LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        let item = sqlx::query_as::<_, (String, String, String)>(
            "SELECT sha256, extension, kind FROM library_items WHERE id = $1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(LibraryError::NotFound)?;
        let projects = projects_using(&mut tx, id).await?;
        if !projects.is_empty() {
            return Err(LibraryError::InUse { projects });
        }
        sqlx::query("DELETE FROM library_items WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let (sha256, extension, kind) = item;
        let kind = Kind::try_from(kind).map_err(LibraryError::Invalid)?;
        let thumbnail = super::thumbnail::path(&self.storage, user, &sha256, kind);
        for file in [self.storage.library_file(user, &sha256, &extension), thumbnail] {
            match std::fs::remove_file(&file) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    eprintln!("scorsese-server: could not remove {}: {error}", file.display());
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Item `id` in `tx`'s scope.
pub(super) async fn read(tx: &mut Tx, id: i64) -> Result<Item, LibraryError> {
    let row: Option<ItemRow> = sqlx::query_as(concat!(
        "SELECT ",
        item_columns!(),
        " FROM library_items WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map_or(Err(LibraryError::NotFound), item)
}

/// The item holding these bytes in `tx`'s scope, as `(id, name)`.
pub(super) async fn holding(tx: &mut Tx, sha256: &str) -> Result<Option<(i64, String)>, sqlx::Error> {
    sqlx::query_as("SELECT id, name FROM library_items WHERE sha256 = $1")
        .bind(sha256)
        .fetch_optional(&mut **tx)
        .await
}

/// The projects in `tx`'s scope that use item `id`.
async fn projects_using(tx: &mut Tx, id: i64) -> Result<Vec<UsedBy>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.id, p.name FROM project_assets pa
         JOIN projects p ON p.id = pa.project_id
         JOIN library_items l ON l.user_id = pa.user_id AND l.sha256 = pa.sha256
         WHERE l.id = $1 ORDER BY p.name, p.id",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await
}

/// `search` with `ILIKE`'s own characters taken literally.
fn escape_like(search: &str) -> String {
    search
        .chars()
        .flat_map(|c| match c {
            '%' | '_' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect()
}
