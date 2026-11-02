//! How much one user may keep, and the hourly sweep that gives back the room
//! abandoned uploads hold.
//!
//! **Each user keeps at most their quota**
//! ([`Storage::quota`](crate::storage::Storage::quota)): their library, their
//! kept renders, their page captures and every upload they have announced,
//! counted at its announced length from the moment it is announced
//! (`upload`). Every decision is made under the user's lock
//! (`credits::ledger::lock`), in the transaction that writes the row it lets
//! in, so two that race cannot both take the last of it. The free tools that
//! add a file — a stock import, a bake — are admitted that way
//! ([`Library::admit_free`], [`Library::keep_bake`]), and so is a render
//! (`renders::job`); a paid generation never asks: it was paid for, and is
//! kept. Every one of them is refused while the disk is below its floor
//! ([`Storage::on_disk`]).
//!
//! **An upload nobody has touched for a day is abandoned.** The same user's
//! next announcement sweeps theirs away, and [`sweep`] sweeps everybody's
//! every hour, so one holds neither disk nor quota for long. Neither takes an
//! upload being written; one swept as its chunk arrives anyway is found gone
//! when the chunk is done, and its bytes are removed (`upload`).

use std::time::Duration;

use tokio::sync::watch;

use super::{Arrival, Item, Library, LibraryError};
use crate::credits::ledger;
use crate::db::{self, Tx, UserId};
use crate::storage::{Storage, bytes_under};

/// How often [`sweep`] looks for abandoned uploads.
const SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);

/// Whether admitting a file asks for its room ([`Library::admit_free`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Room {
    /// Asked in the transaction that admits it: a free tool's file.
    Asked,
    /// Already settled: an upload, whose announcement took its room, or paid
    /// output, which is kept whatever it takes.
    Settled,
}

/// What `user` keeps now, in bytes: their library, their kept renders, their
/// page captures and their announced uploads, but for upload `except`.
///
/// Refused as [`LibraryError::DiskLow`] first while the disk is below its
/// floor, since whatever asks is about to write. Takes `user`'s lock
/// (`credits::ledger::lock`), held until `tx` ends, so a decision made on
/// this and the row it lets in are one.
pub(crate) async fn kept(
    tx: &mut Tx,
    storage: &Storage,
    user: UserId,
    except: Option<i64>,
) -> Result<u64, LibraryError> {
    let (disk, pages) = {
        let storage = storage.clone();
        crate::jobs::blocking("measuring the disk crashed", move || {
            (storage.on_disk(), bytes_under(&storage.page_cache(user)))
        })
        .await
        .map_err(LibraryError::Tools)?
    };
    disk?;
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
    Ok(used.unsigned_abs().saturating_add(pages))
}

/// Refused as [`LibraryError::Full`] unless `user` has room for `needed`
/// more bytes beside what they keep ([`kept`], whose lock it takes).
pub(crate) async fn room_for(
    tx: &mut Tx,
    storage: &Storage,
    user: UserId,
    needed: u64,
    except: Option<i64>,
) -> Result<(), LibraryError> {
    let used = kept(tx, storage, user, except).await?;
    let quota = storage.quota().get();
    if used.saturating_add(needed) > quota {
        return Err(LibraryError::Full {
            used,
            quota,
            needed,
        });
    }
    Ok(())
}

impl Library {
    /// Check `arrival`, a free tool's file (a stock import), and make it
    /// `user`'s item — refused as [`LibraryError::Full`] unless they have
    /// room for it, decided in the transaction that admits it.
    pub async fn admit_free(&self, user: UserId, arrival: Arrival) -> Result<Item, LibraryError> {
        self.admit_with(user, arrival, Room::Asked).await
    }

    /// [`Library::keep_generated`] for a free bake: refused as
    /// [`LibraryError::Full`] unless `user` has room for it, decided in the
    /// transaction that admits it.
    pub async fn keep_bake(
        &self,
        user: UserId,
        brief_hash: &str,
        arrival: Arrival,
    ) -> Result<Item, LibraryError> {
        self.keep(user, brief_hash, arrival, Room::Asked).await
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
