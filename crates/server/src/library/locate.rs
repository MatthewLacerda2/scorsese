//! Where a user's files are on disk — what laying a stored project out as a
//! `.scor` folder needs (`projects::media::materialise`).
//!
//! Two questions, both answered from the user's own `library_items`, in the
//! caller's scoped transaction, so a document naming somebody else's hash or
//! brief finds nothing:
//!
//! - [`by_hash`]: the file each asset's `sha256` names — every file a project
//!   uses, uploaded or generated — with a spoken line's word timings where
//!   its item keeps them (#886).
//! - [`by_brief`]: the generated file each brief hash made — so a brief
//!   already paid for, in this project or another of the same user's, is
//!   laid out where the brief would land, and `scorsese_providers` finds it
//!   there exactly as it finds a local project's `generated/` file: already
//!   generated, nothing to pay (#539).

use std::collections::HashMap;
use std::path::PathBuf;

use scorsese_core::words::Words;

use super::{Item, Library, LibraryError, store};
use crate::db::{self, Tx, UserId};
use crate::projects::media::MediaSource;
use crate::storage::Storage;

/// A user's files by hash, as [`by_hash`] found them: what laying a project
/// out reads.
#[derive(Debug, Clone, Default)]
pub struct Located {
    files: HashMap<String, PathBuf>,
    words: HashMap<String, Words>,
}

impl MediaSource for Located {
    fn locate(&self, sha256: &str) -> Option<PathBuf> {
        self.files.get(sha256).cloned()
    }

    fn words(&self, sha256: &str) -> Option<Words> {
        self.words.get(sha256).cloned()
    }
}

/// The files among `hashes` that `user` has, by hash, and the word timings
/// kept with any of them.
pub async fn by_hash(
    tx: &mut Tx,
    storage: &Storage,
    user: UserId,
    hashes: &[String],
) -> Result<Located, sqlx::Error> {
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT sha256, extension, words::text FROM library_items WHERE sha256 = ANY($1)",
    )
    .bind(hashes)
    .fetch_all(&mut **tx)
    .await?;
    let mut located = Located::default();
    for (hash, extension, words) in rows {
        // Timings that will not read are none: said, as a line without any
        // is, rather than failing the whole folder.
        if let Some(words) = words.and_then(|text| serde_json::from_str(&text).ok()) {
            located.words.insert(hash.clone(), words);
        }
        let path = storage.library_file(user, &hash, &extension);
        located.files.insert(hash, path);
    }
    Ok(located)
}

/// The generated files among `briefs` that `user` has, by brief hash.
pub async fn by_brief(
    tx: &mut Tx,
    storage: &Storage,
    user: UserId,
    briefs: &[String],
) -> Result<HashMap<String, PathBuf>, sqlx::Error> {
    let files: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT brief_hash, sha256, extension FROM library_items WHERE brief_hash = ANY($1)",
    )
    .bind(briefs)
    .fetch_all(&mut **tx)
    .await?;
    Ok(files
        .into_iter()
        .map(|(brief, hash, extension)| {
            let path = storage.library_file(user, &hash, &extension);
            (brief, path)
        })
        .collect())
}

impl Library {
    /// `user`'s items narrowed by `filter`, newest first, with everything
    /// known about each — what an assistant choosing a file reads, where a
    /// list's tiles need only [`Summary`](super::Summary).
    pub async fn catalogue(
        &self,
        user: UserId,
        filter: &super::Filter,
    ) -> Result<Vec<Item>, LibraryError> {
        let listed = self.list(user, filter).await?;
        let mut tx = db::scoped(&self.pool, user).await?;
        let mut items = Vec::with_capacity(listed.len());
        for summary in listed {
            items.push(store::read(&mut tx, summary.id).await?);
        }
        tx.commit().await?;
        Ok(items)
    }
}
