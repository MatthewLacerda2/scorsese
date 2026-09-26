//! Letting a file into the library: the one way an item comes to exist.
//!
//! Uploads and generated output both end here, so the checks cannot differ
//! between them: the bytes are hashed by the server, probed and held to their
//! kind by `scorsese_core::pool` exactly as `scorsese import` holds a file,
//! moved to where the hash says, recorded, and given a thumbnail job — all
//! before anyone can see the item.

use std::io;
use std::path::{Path, PathBuf};

use scorsese_core::ImportError;
use scorsese_core::pool::{hash_file, measure};
use scorsese_render::{Ffprobe, Tools};
use serde_json::json;

use super::store::{self, ItemRow, item_columns};
use super::{Item, Kind, Library, LibraryError};
use crate::db::{self, UserId};
use crate::jobs::{kinds, store as jobs};

/// A file on this machine that is to become a library item.
#[derive(Debug, Clone)]
pub struct Arrival {
    /// Where its bytes are now. Consumed by [`Library::admit`] either way:
    /// moved into the library, or removed.
    pub file: PathBuf,
    /// What the user will see it called.
    pub name: String,
    /// What it claims to be.
    pub kind: Kind,
    /// Its extension, lower case.
    pub extension: String,
    /// The hash the sender announced, which the bytes must have.
    pub announced: Option<String>,
    /// For generated output, the hash of its brief.
    pub brief_hash: Option<String>,
}

/// What reading a file found.
struct Measured {
    sha256: String,
    size: u64,
    media: String,
}

impl Library {
    /// Check `arrival`, and make it `user`'s item.
    ///
    /// [`LibraryError::Duplicate`] when the user already has these bytes,
    /// [`LibraryError::Rejected`] when they are not what they claim.
    pub async fn admit(&self, user: UserId, arrival: Arrival) -> Result<Item, LibraryError> {
        let file = arrival.file.clone();
        let outcome = self.admit_file(user, arrival).await;
        if outcome.is_err() {
            // Whatever did not get in is not kept: a half-checked file is not
            // anything the user can find again.
            let _ = std::fs::remove_file(&file);
        }
        outcome
    }

    async fn admit_file(&self, user: UserId, arrival: Arrival) -> Result<Item, LibraryError> {
        let measured = {
            let (tools, file, kind) = (self.tools.clone(), arrival.file.clone(), arrival.kind);
            tokio::task::spawn_blocking(move || read(&tools, &file, kind))
                .await
                .map_err(|error| LibraryError::Tools(error.to_string()))??
        };
        if let Some(announced) = &arrival.announced
            && *announced != measured.sha256
        {
            return Err(LibraryError::Rejected(
                "the file that arrived is not the one announced (its hash differs); \
                 upload it again"
                    .to_owned(),
            ));
        }

        let mut tx = db::scoped(&self.pool, user).await?;
        if let Some((id, name)) = store::holding(&mut tx, &measured.sha256).await? {
            return Err(LibraryError::Duplicate { id, name });
        }
        let row: ItemRow = sqlx::query_as(concat!(
            "INSERT INTO library_items
                 (user_id, sha256, name, kind, extension, size_bytes, media, brief_hash)
             VALUES (member_id(), $1, $2, $3, $4, $5, $6::jsonb, $7) RETURNING ",
            item_columns!()
        ))
        .bind(&measured.sha256)
        .bind(arrival.name.trim())
        .bind(arrival.kind.as_str())
        .bind(&arrival.extension)
        .bind(i64::try_from(measured.size).unwrap_or(i64::MAX))
        .bind(&measured.media)
        .bind(&arrival.brief_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| match &error {
            // The same bytes, or the same brief, arriving twice at once: the
            // check above passed for both, and the second is the one refused.
            sqlx::Error::Database(refused) if refused.is_unique_violation() => {
                LibraryError::Rejected(
                    "this file arrived twice at once, and the other copy is in your library"
                        .to_owned(),
                )
            }
            _ => LibraryError::Database(error),
        })?;
        let item = store::item(row)?;
        let job = jobs::enqueue(&mut tx, kinds::THUMBNAIL, &json!({ "item": item.id })).await?;
        let home = self
            .storage
            .library_file(user, &measured.sha256, &arrival.extension);
        move_file(&arrival.file, &home)?;
        tx.commit().await?;
        self.queue.announce(user, &job);
        Ok(item)
    }
}

/// Hash and probe `file`, and hold it to `kind`.
fn read(tools: &Tools, file: &Path, kind: Kind) -> Result<Measured, LibraryError> {
    let sha256 = hash_file(file)?;
    let size = file.metadata()?.len();
    let media = measure(file, kind.asset_kind(), &Ffprobe::new(tools.clone())).map_err(
        |error| match error {
            ImportError::KindMismatch { found, .. } => LibraryError::Rejected(format!(
                "this was sent as {} but has {found}",
                article(kind)
            )),
            other => {
                // The prober's own words name the server's paths; they are
                // for the log, not for the person uploading.
                eprintln!("scorsese-server: refusing an upload: {other}");
                LibraryError::Rejected(format!(
                    "this could not be read as {}; is it damaged, or not really a {} file?",
                    article(kind),
                    kind.as_str()
                ))
            }
        },
    )?;
    let media = serde_json::to_string(&media)
        .map_err(|error| LibraryError::Invalid(format!("the probed media: {error}")))?;
    Ok(Measured {
        sha256,
        size,
        media,
    })
}

/// "a video", "an image", "a sound".
fn article(kind: Kind) -> &'static str {
    match kind {
        Kind::Video => "a video",
        Kind::Image => "an image",
        Kind::Audio => "a sound",
    }
}

/// Move `from` to `to`, across filesystems if it must.
///
/// The cache and the library are separate roots and may be separate disks; a
/// rename cannot cross one, so that case copies and removes.
fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(from, to) {
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            std::fs::copy(from, to)?;
            std::fs::remove_file(from)
        }
        other => other,
    }
}
