//! Generated output in the library, found again by its brief.
//!
//! A Veo shot, a spoken line or a synthesis bake is a library item like an
//! upload, carrying the hash of the brief that made it — the same digest
//! `scorsese_providers` addresses a local project's `generated/` by
//! (`video::Brief::digest`, `speech::Brief::digest`). A generation job asks
//! [`Library::find_generated`] before it spends anything, and keeps what it
//! made with [`Library::keep_generated`].
//!
//! Scoped like everything else here, so the lookup **cannot** find another
//! user's output: a brief two people happen to share is paid for by each
//! (#527). The jobs that generate are `crate::generations` (#539): each pays
//! through `credits::generations` (#537), asks `find_generated` before it
//! sends anything, keeps its output with `keep_generated`, and settles with
//! `Answer::Worked(Some(item.id))` — which is what links the audit row to the
//! item ([`Library::generation`] reads it back). Laying a project out for a
//! tool (`tools::lay_out`) asks the same question by brief, through
//! `library::locate`, so a brief already made is free in any of the user's
//! projects.

use scorsese_core::words::Words;
use serde_json::Value;

use super::{Arrival, Item, Kind, Library, LibraryError, store};
use crate::db::{self, UserId};

/// What a generation made, as bytes, before the library keeps it.
pub struct Made {
    /// The output itself.
    pub bytes: Vec<u8>,
    /// What it is; its extension follows ([`Kind::generated_extension`]).
    pub kind: Kind,
    /// What the item is called, without the extension.
    pub name: String,
    /// A spoken line's word timings (#886), `None` for output that has none.
    pub words: Option<Words>,
}

/// Why [`Library::keep_generated_bytes`] kept nothing.
#[derive(Debug)]
pub enum Unkept {
    /// The bytes could not be written to the user's scratch folder.
    Unwritten(std::io::Error),
    /// The library refused them.
    Refused(LibraryError),
}

impl Library {
    /// `user`'s item generated from the brief with this hash, if they have one.
    pub async fn find_generated(
        &self,
        user: UserId,
        brief_hash: &str,
    ) -> Result<Option<Item>, LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        let id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM library_items WHERE brief_hash = $1")
                .bind(brief_hash)
                .fetch_optional(&mut *tx)
                .await?;
        let item = match id {
            Some(id) => Some(store::read(&mut tx, id).await?),
            None => None,
        };
        tx.commit().await?;
        Ok(item)
    }

    /// Keep what a generation made as `user`'s item, under `brief_hash`.
    ///
    /// `arrival.brief_hash` is set from `brief_hash`. When the user already has
    /// these bytes — two briefs that came out identical — the item they have is
    /// the answer rather than a refusal: output is not an upload somebody could
    /// have chosen not to send twice.
    pub async fn keep_generated(
        &self,
        user: UserId,
        brief_hash: &str,
        arrival: Arrival,
    ) -> Result<Item, LibraryError> {
        let arrival = Arrival {
            brief_hash: Some(brief_hash.to_owned()),
            ..arrival
        };
        match self.admit(user, arrival).await {
            Err(LibraryError::Duplicate { id, .. }) => self.get(user, id).await,
            kept => kept,
        }
    }

    /// Keep `made` as `user`'s item under `brief_hash`: written to the
    /// user's scratch folder, then [`Library::keep_generated`].
    pub async fn keep_generated_bytes(
        &self,
        user: UserId,
        brief_hash: &str,
        made: Made,
    ) -> Result<Item, Unkept> {
        let extension = made.kind.generated_extension();
        let file = self.storage.scratch(user).with_extension(extension);
        file.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, &made.bytes))
            .map_err(Unkept::Unwritten)?;
        let arrival = Arrival {
            file,
            name: format!("{}.{extension}", made.name),
            kind: made.kind,
            extension: extension.to_owned(),
            announced: None,
            brief_hash: None,
            words: made.words,
        };
        self.keep_generated(user, brief_hash, arrival)
            .await
            .map_err(Unkept::Refused)
    }

    /// The record of the generation that made `user`'s item `id` — brief,
    /// model, settings, when, and what it cost — or `None` for an upload.
    ///
    /// So "what did this cost?" is answered from the asset itself, by the user
    /// and the assistant alike (#535, #537). `estimated_cost_micros` is the
    /// provider's price by scorsese's own table (no provider reports one,
    /// `docs/prices.md`); `charged_micros` is what the ledger took for it.
    pub async fn generation(&self, user: UserId, id: i64) -> Result<Option<Value>, LibraryError> {
        let mut tx = db::scoped(&self.pool, user).await?;
        let record = sqlx::query_scalar(include_str!("generation.sql"))
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(record)
    }
}
