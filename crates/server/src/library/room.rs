//! How much one user may keep, and the hourly sweep that gives back the room
//! abandoned uploads hold.
//!
//! **Each user keeps at most their quota**
//! ([`Storage::quota`](crate::storage::Storage::quota)): their library, their
//! kept renders and every upload they have announced, counted at its
//! announced length from the moment it is announced (`upload`). Every
//! decision is made under the user's lock (`credits::ledger::lock`), so two
//! that race cannot both take the last of it. The free tools that add a file
//! — a stock import, a bake — ask first ([`Library::make_room`]); a paid
//! generation never does: it was paid for, and is kept.
//!
//! **An upload nobody has touched for a day is abandoned.** The same user's
//! next announcement sweeps theirs away, and [`sweep`] sweeps everybody's
//! every hour, so one holds neither disk nor quota for long. Neither takes an
//! upload being written; one swept as its chunk arrives anyway is found gone
//! when the chunk is done, and its bytes are removed (`upload`).

use std::time::Duration;

use tokio::sync::watch;

use super::{Library, LibraryError};
use crate::credits::ledger;
use crate::db::{self, Tx, UserId};

/// How often [`sweep`] looks for abandoned uploads.
const SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);

impl Library {
    /// Refused as [`LibraryError::Full`] unless `user` has room for a file of
    /// `bytes` beside what they keep — for a free tool's file (a stock
    /// import, a bake) about to be admitted. A paid generation is never asked:
    /// it was paid for, and is kept.
    pub async fn make_room(&self, user: UserId, bytes: u64) -> Result<(), LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        self.room(&mut tx, i64::try_from(bytes).unwrap_or(i64::MAX), None)
            .await?;
        Ok(tx.commit().await?)
    }

    /// Refused as [`LibraryError::Full`] unless the user `tx` is scoped to has
    /// room for `length` more bytes beside what they keep: their library,
    /// their kept renders and their announced uploads, but for upload
    /// `except` — the one the bytes are for. Takes the user's lock, held until
    /// `tx` ends, so a decision and the row it lets in are one.
    pub(super) async fn room(
        &self,
        tx: &mut Tx,
        length: i64,
        except: Option<i64>,
    ) -> Result<(), LibraryError> {
        ledger::lock(tx).await?;
        let used: i64 = sqlx::query_scalar(
            "SELECT ((SELECT coalesce(sum(size_bytes), 0) FROM library_items)
                   + (SELECT coalesce(sum(size), 0) FROM renders)
                   + (SELECT coalesce(sum(length), 0) FROM uploads
                      WHERE id IS DISTINCT FROM $1))::bigint",
        )
        .bind(except)
        .fetch_one(&mut **tx)
        .await?;
        let (used, needed) = (used.unsigned_abs(), length.unsigned_abs());
        let quota = self.storage.quota().get();
        if used.saturating_add(needed) > quota {
            return Err(LibraryError::Full {
                used,
                quota,
                needed,
            });
        }
        Ok(())
    }

    /// Abandon every user's upload nobody has touched for a day, and what of
    /// it arrived — but one a request is writing right now; how many.
    /// Privileged: what has been abandoned is a question about everybody
    /// (`db::scope`'s list), and the files are removed as each row's owner's.
    pub async fn sweep_uploads(&self) -> Result<usize, LibraryError> {
        let writing: Vec<i64> = self.uploading.ids();
        let mut tx = db::privileged(&self.pool).await?;
        let abandoned: Vec<(i64, i64, String)> = sqlx::query_as(
            "DELETE FROM uploads
             WHERE touched_at < now() - interval '1 day' AND NOT (id = ANY($1))
             RETURNING user_id, id, extension",
        )
        .bind(&writing)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        for (user, id, extension) in &abandoned {
            let file = self
                .storage
                .upload_file(UserId::from_row(*user), *id, extension);
            let _ = std::fs::remove_file(file);
        }
        Ok(abandoned.len())
    }
}

/// Sweep every user's abandoned uploads ([`Library::sweep_uploads`]) every
/// hour, until `stop`.
pub async fn sweep(library: Library, mut stop: watch::Receiver<bool>) {
    loop {
        match library.sweep_uploads().await {
            Ok(0) => {}
            Ok(swept) => tracing::info!("swept {swept} abandoned uploads"),
            Err(error) => tracing::error!("the upload sweep failed: {error}"),
        }
        tokio::select! {
            _ = stop.wait_for(|stopping| *stopping) => return,
            () = tokio::time::sleep(SWEEP_EVERY) => {}
        }
    }
}
