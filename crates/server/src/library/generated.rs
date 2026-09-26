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
//! (#527). The jobs that generate are not written yet: one pays through
//! `credits::generations` (#537), asks `find_generated` before `start`, keeps
//! its output with `keep_generated`, and settles with
//! `Answer::Worked(Some(item.id))` — which is what links the audit row to the
//! item ([`Library::generation`] reads it back). This is the library's side of
//! that contract, and nothing calls it yet.

use serde_json::Value;

use super::{Arrival, Item, Library, LibraryError, store};
use crate::db::{self, UserId};

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
