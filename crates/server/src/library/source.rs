//! Where a stock file came from, kept on its item (#1237).
//!
//! The asset's `source` block is the record — it travels with the project.
//! The item keeps a copy, written once when the file is kept
//! ([`crate::tools::fetched::keep`]), so the details panel can say where a
//! file came from without reading any project.

use scorsese_core::Source;

use super::{Library, LibraryError};
use crate::db::{self, UserId};

impl Library {
    /// Records that `user`'s item `id` came from `source`, unless it already
    /// says where it came from: the first origin a file was kept with stays.
    pub async fn record_source(
        &self,
        user: UserId,
        id: i64,
        source: &Source,
    ) -> Result<(), LibraryError> {
        let written = serde_json::to_string(source)
            .map_err(|error| LibraryError::Invalid(format!("a source: {error}")))?;
        let mut tx = db::scoped(&self.pool, user).await?;
        sqlx::query("UPDATE library_items SET source = $2::jsonb WHERE id = $1 AND source IS NULL")
            .bind(id)
            .bind(written)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Where `user`'s item `id` came from, or `None` for a file nobody
    /// recorded an origin for — an upload, or a generation, whose record is
    /// [`Library::generation`].
    pub async fn source(&self, user: UserId, id: i64) -> Result<Option<Source>, LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        let written: Option<Option<String>> =
            sqlx::query_scalar("SELECT source::text FROM library_items WHERE id = $1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        tx.commit().await?;
        written
            .flatten()
            .map(|text| serde_json::from_str(&text))
            .transpose()
            .map_err(|error| LibraryError::Invalid(format!("item {id}'s source: {error}")))
    }
}
