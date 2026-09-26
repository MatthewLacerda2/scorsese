//! Uploads that arrive in pieces and survive a dropped connection: the
//! storage half of tus (`http::uploads` is the protocol half).
//!
//! An upload is **announced** — name, size and the hash the browser computed —
//! and refused there if the user already has those bytes or if the name is not
//! a kind of file scorsese edits, so neither costs a byte of transfer. Then its
//! bytes are **appended** in chunks, each under the Cloudflare tunnel's 100 MB
//! request cap, to a file in the cache; how far it got is that file's length,
//! so an interrupted chunk keeps whatever reached the disk and the next one
//! resumes from there. The last chunk **admits** the file
//! ([`Library::admit`]): hashed again by the server, probed, and moved into the
//! library.
//!
//! One writer per upload at a time. A second request appending to an upload
//! already being written is refused as [`LibraryError::Busy`] rather than
//! queued behind it — two writers would each check the offset and then both
//! append. The guard is in memory, which is enough: there is one server, and a
//! restart ends every request that held one.
//!
//! An upload nobody has touched for a day is abandoned. The next announcement
//! by the same user sweeps theirs away, so no timer is needed and nothing
//! crosses users.

use std::collections::HashSet;
use std::fmt::Display;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use futures_util::{Stream, StreamExt};
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use super::{Arrival, Item, Kind, Library, LibraryError, store};
use crate::db::{self, UserId};

/// The largest file an upload may announce: 20 GiB, a long screen recording
/// with room to spare.
pub const MAX_UPLOAD_BYTES: i64 = 20 * 1024 * 1024 * 1024;

/// What an upload says it will be, before any of its bytes are sent.
#[derive(Debug, Clone)]
pub struct Announced {
    /// The file's name as the user had it; its extension decides its kind.
    pub name: String,
    /// The hash the browser computed. Checked against the bytes at the end.
    pub sha256: String,
    /// How many bytes it will be.
    pub length: i64,
}

/// How far an upload has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// Bytes received.
    pub offset: i64,
    /// Bytes announced.
    pub length: i64,
}

/// What appending a chunk came to.
#[derive(Debug)]
pub enum Appended {
    /// More to come.
    Partial(Progress),
    /// That was the last of it, and the file is in the library.
    Finished(Box<Item>),
}

/// The uploads being written right now.
#[derive(Debug, Clone, Default)]
pub(super) struct InFlight(Arc<Mutex<HashSet<i64>>>);

/// Held while one request writes an upload; letting go frees it.
struct Writing(InFlight, i64);

impl InFlight {
    fn claim(&self, id: i64) -> Option<Writing> {
        let mut writing = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        writing.insert(id).then(|| Writing(self.clone(), id))
    }
}

impl Drop for Writing {
    fn drop(&mut self) {
        let mut writing = (self.0).0.lock().unwrap_or_else(PoisonError::into_inner);
        writing.remove(&self.1);
    }
}

/// An upload's row: name, kind, extension, hash, length.
type UploadRow = (String, String, String, String, i64);

impl Library {
    /// Announce an upload for `user`; its id.
    pub async fn start_upload(
        &self,
        user: UserId,
        announced: &Announced,
    ) -> Result<i64, LibraryError> {
        let name = announced.name.trim();
        let kind =
            Kind::of_file_name(name).ok_or_else(|| LibraryError::Unsupported(name.to_owned()))?;
        let extension = extension_of(name);
        let sha256 = announced.sha256.trim().to_ascii_lowercase();
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(LibraryError::Invalid(
                "an upload names its SHA-256, as 64 hex digits".to_owned(),
            ));
        }
        if !(1..=MAX_UPLOAD_BYTES).contains(&announced.length) {
            return Err(LibraryError::Invalid(format!(
                "a file must be between one byte and {} GiB",
                MAX_UPLOAD_BYTES >> 30
            )));
        }

        let mut tx = db::scoped(&self.pool, user).await?;
        if let Some((id, name)) = store::holding(&mut tx, &sha256).await? {
            return Err(LibraryError::Duplicate { id, name });
        }
        let abandoned: Vec<(i64, String)> = sqlx::query_as(
            "DELETE FROM uploads WHERE touched_at < now() - interval '1 day'
             RETURNING id, extension",
        )
        .fetch_all(&mut *tx)
        .await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO uploads (user_id, name, kind, extension, sha256, length)
             VALUES (member_id(), $1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(name)
        .bind(kind.as_str())
        .bind(&extension)
        .bind(&sha256)
        .bind(announced.length)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        for (old, extension) in abandoned {
            let _ = std::fs::remove_file(self.storage.upload_file(user, old, &extension));
        }
        let file = self.storage.upload_file(user, id, &extension);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::File::create(file)?;
        Ok(id)
    }

    /// How far `user`'s upload `id` has got.
    pub async fn upload_progress(&self, user: UserId, id: i64) -> Result<Progress, LibraryError> {
        let (_, _, extension, _, length) = self.upload(user, id).await?;
        let offset = received(&self.storage.upload_file(user, id, &extension));
        Ok(Progress { offset, length })
    }

    /// Append `body` to `user`'s upload `id`, which must have received exactly
    /// `offset` bytes so far. The last chunk admits the file.
    pub async fn append<S, B, E>(
        &self,
        user: UserId,
        id: i64,
        offset: i64,
        body: S,
    ) -> Result<Appended, LibraryError>
    where
        S: Stream<Item = Result<B, E>> + Unpin,
        B: AsRef<[u8]>,
        E: Display,
    {
        let (name, kind, extension, sha256, length) = self.upload(user, id).await?;
        let _writing = self.uploading.claim(id).ok_or(LibraryError::Busy)?;
        let path = self.storage.upload_file(user, id, &extension);
        let mut at = received(&path);
        if offset != at {
            return Err(LibraryError::Offset { expected: at });
        }
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await?;
        let mut body = body;
        let mut failed = None;
        while let Some(chunk) = body.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    failed = Some(LibraryError::Invalid(format!(
                        "the upload broke off: {error}"
                    )));
                    break;
                }
            };
            let bytes = chunk.as_ref();
            let after = at + i64::try_from(bytes.len()).unwrap_or(i64::MAX);
            if after > length {
                failed = Some(LibraryError::Invalid(
                    "more bytes arrived than the upload announced".to_owned(),
                ));
                break;
            }
            file.write_all(bytes).await?;
            at = after;
        }
        file.flush().await?;
        self.touch(user, id).await?;
        if let Some(failed) = failed {
            return Err(failed);
        }
        if at < length {
            return Ok(Appended::Partial(Progress { offset: at, length }));
        }

        let kind = Kind::try_from(kind).map_err(LibraryError::Invalid)?;
        let arrival = Arrival {
            file: path,
            name,
            kind,
            extension,
            announced: Some(sha256),
            brief_hash: None,
        };
        let admitted = self.admit(user, arrival).await;
        // Finished either way: in the library, or refused and removed.
        self.forget_upload(user, id).await?;
        admitted.map(|item| Appended::Finished(Box::new(item)))
    }

    /// Abandon `user`'s upload `id` and whatever of it arrived.
    pub async fn cancel_upload(&self, user: UserId, id: i64) -> Result<(), LibraryError> {
        let (_, _, extension, _, _) = self.upload(user, id).await?;
        let _writing = self.uploading.claim(id).ok_or(LibraryError::Busy)?;
        self.forget_upload(user, id).await?;
        let _ = std::fs::remove_file(self.storage.upload_file(user, id, &extension));
        Ok(())
    }

    async fn upload(&self, user: UserId, id: i64) -> Result<UploadRow, LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        let row = sqlx::query_as(
            "SELECT name, kind, extension, sha256, length FROM uploads WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        row.ok_or(LibraryError::NotFound)
    }

    async fn touch(&self, user: UserId, id: i64) -> Result<(), LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        sqlx::query("UPDATE uploads SET touched_at = now() WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        Ok(tx.commit().await?)
    }

    async fn forget_upload(&self, user: UserId, id: i64) -> Result<(), LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        sqlx::query("DELETE FROM uploads WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        Ok(tx.commit().await?)
    }
}

/// The bytes an upload's file holds so far; none when it is missing.
fn received(file: &Path) -> i64 {
    file.metadata()
        .map_or(0, |meta| i64::try_from(meta.len()).unwrap_or(i64::MAX))
}

/// `name`'s extension, lower case. Always present: a name without one has no
/// kind, and was refused before this is asked.
fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}
