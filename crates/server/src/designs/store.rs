//! Reading a user's designs and the voices they kept, and recording a kept
//! one. Every function takes a transaction scoped to the user.

use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use scorsese_providers::voices::design::Brief;

use crate::db::Tx;

/// One candidate of a design, as `voice_designs.candidates` holds it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// The vendor's handle: what keeping it is asked with.
    pub generated_voice_id: String,
    /// The hash its sample is kept under in the library
    /// ([`sample_hash`](super::sample_hash)).
    pub sample: String,
    /// How long it runs, where the vendor said.
    #[serde(default)]
    pub seconds: Option<f64>,
}

/// A design that worked.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Design {
    /// Its row.
    pub id: i64,
    /// The description.
    pub prompt: String,
    /// What the candidates read.
    pub passage: String,
    /// The seed, where one was given.
    pub seed: Option<i64>,
    /// How literally the description was followed, where that was set.
    pub guidance: Option<f64>,
    /// The candidates, in the order they came back.
    pub candidates: Json<Vec<Sample>>,
}

impl Design {
    /// The brief it was designed from — what keeping a candidate sends with it.
    pub fn brief(&self) -> Brief {
        Brief {
            prompt: self.prompt.clone(),
            passage: self.passage.clone(),
            seed: self.seed.and_then(|seed| u32::try_from(seed).ok()),
            guidance: self.guidance,
        }
    }

    /// The candidates other than `chosen`: heard and passed over.
    pub fn passed_over(&self, chosen: &str) -> Vec<String> {
        self.candidates
            .iter()
            .map(|sample| sample.generated_voice_id.clone())
            .filter(|id| id != chosen)
            .collect()
    }
}

/// The newest design that worked from the brief with this hash.
pub async fn find(tx: &mut Tx, brief_hash: &str) -> Result<Option<Design>, sqlx::Error> {
    select(tx, None, Some(brief_hash), None).await
}

/// The design `id`, if it worked.
pub async fn get(tx: &mut Tx, id: i64) -> Result<Option<Design>, sqlx::Error> {
    select(tx, Some(id), None, None).await
}

/// The newest design that offered the candidate `generated_voice_id`.
pub async fn offering(
    tx: &mut Tx,
    generated_voice_id: &str,
) -> Result<Option<Design>, sqlx::Error> {
    select(tx, None, None, Some(generated_voice_id)).await
}

/// The newest design that worked and matches every filter given.
async fn select(
    tx: &mut Tx,
    id: Option<i64>,
    brief_hash: Option<&str>,
    offered: Option<&str>,
) -> Result<Option<Design>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, prompt, passage, seed, guidance, candidates FROM voice_designs
         WHERE state = 'generated'
           AND ($1::bigint IS NULL OR id = $1)
           AND ($2::text IS NULL OR brief_hash = $2)
           AND ($3::text IS NULL OR candidates @> jsonb_build_array(
                    jsonb_build_object('generated_voice_id', $3::text)))
         ORDER BY id DESC LIMIT 1",
    )
    .bind(id)
    .bind(brief_hash)
    .bind(offered)
    .fetch_optional(&mut **tx)
    .await
}

/// A voice the user kept, and what made it.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Kept {
    /// The id a narration names.
    pub voice_id: String,
    /// What it was called when it was kept.
    pub name: String,
    /// The description it was designed from.
    pub prompt: String,
    /// The seed it was designed with, where there was one.
    pub seed: Option<i64>,
    /// The design it came out of.
    pub design: i64,
    /// What that design was estimated to cost, before the markup.
    pub estimated_cost_micros: i64,
}

/// Record that `voice_id`, called `name`, was kept out of `design`'s
/// candidate `chosen` at `provider`.
pub(super) async fn keep(
    tx: &mut Tx,
    design: i64,
    chosen: &str,
    voice: (&str, &str),
    provider: &str,
) -> Result<(), sqlx::Error> {
    let (voice_id, name) = voice;
    sqlx::query(
        "INSERT INTO designed_voices (user_id, design_id, voice_id, name, generated_voice_id,
                                      provider)
         VALUES (member_id(), $1, $2, $3, $4, $5)",
    )
    .bind(design)
    .bind(voice_id)
    .bind(name)
    .bind(chosen)
    .bind(provider)
    .execute(&mut **tx)
    .await
    .map(drop)
}

/// Every voice the user designed and kept, oldest first.
pub async fn designed(tx: &mut Tx) -> Result<Vec<Kept>, sqlx::Error> {
    sqlx::query_as(
        "SELECT v.voice_id, v.name, d.prompt, d.seed, d.id AS design, d.estimated_cost_micros
         FROM designed_voices v JOIN voice_designs d ON d.id = v.design_id
         ORDER BY v.id",
    )
    .fetch_all(&mut **tx)
    .await
}
