//! A voice design's audit row (#572): `voice_designs`, the fourth kind of
//! paid generation beside a shot, a still and a line.
//!
//! Started and settled by [`generations`](super::generations) like the other
//! three — reserved before ElevenLabs is asked, charged once it answers, free
//! when it refuses. What only a design has is written here: one call is billed
//! **once**, for the passage, and answers with three candidates
//! (`scorsese_providers::voices::design::price`), so the row records the
//! passage's length and, once it worked, the candidates
//! ([`crate::designs`] reads them back).

use serde_json::Value;

use crate::db::Tx;

/// A design about to be asked for, with everything its price depends on.
#[derive(Debug, Clone)]
pub struct Design<'a> {
    /// The project it was asked for in, if any: a voice is the user's.
    pub project: Option<i64>,
    /// The tool call that asked for it.
    pub tool_call: Option<i64>,
    /// The job that runs it.
    pub job: Option<i64>,
    /// What the voice should be like.
    pub prompt: &'a str,
    /// What the candidates read — the only thing billed.
    pub passage: &'a str,
    /// The vendor's best-effort determinism knob.
    pub seed: Option<u32>,
    /// How literally the description is followed.
    pub guidance: Option<f64>,
    /// `Brief::digest`: what the design is found again by.
    pub brief_hash: &'a str,
    /// `voices::design::estimate`'s cents: the quoted figure.
    pub estimated_cents: u64,
}

/// The ledger's words for it.
pub(super) fn memo(design: &Design<'_>) -> String {
    format!(
        "Voice design: {} characters of preview text, three candidates",
        design.passage.chars().count()
    )
}

/// The audit row of a design.
pub(super) async fn insert(
    tx: &mut Tx,
    design: &Design<'_>,
    cost: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO voice_designs (user_id, project_id, tool_call_id, job_id, prompt, passage,
             seed, guidance, characters, brief_hash, estimated_cost_micros)
         VALUES (member_id(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(design.project)
    .bind(design.tool_call)
    .bind(design.job)
    .bind(design.prompt)
    .bind(design.passage)
    .bind(design.seed.map(i64::from))
    .bind(design.guidance)
    .bind(i32::try_from(design.passage.chars().count()).unwrap_or(i32::MAX))
    .bind(design.brief_hash)
    .bind(cost)
    .fetch_one(&mut **tx)
    .await
}

/// Record what design `id` came back with — before it is settled, in the same
/// transaction, so a design charged for always says what it bought.
pub async fn candidates(tx: &mut Tx, id: i64, candidates: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE voice_designs SET candidates = $2 WHERE id = $1")
        .bind(id)
        .bind(candidates)
        .execute(&mut **tx)
        .await
        .map(drop)
}
