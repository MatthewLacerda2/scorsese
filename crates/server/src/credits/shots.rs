//! A Veo shot's audit row: `veo_generations`, started and settled by
//! [`generations`](super::generations) like every paid generation, and the
//! only one paid from the paid balance alone (#1530). A shot is priced by its
//! seconds, model and resolution (`scorsese_providers::prices::estimate`),
//! and its row keeps Google's ticket once it is accepted.

use crate::db::Tx;

/// A Veo shot about to be submitted, with everything its price depends on.
#[derive(Debug, Clone)]
pub struct Shot<'a> {
    /// The project it is for.
    pub project: Option<i64>,
    /// The assistant tool call that asked for it (#540).
    pub tool_call: Option<i64>,
    /// The job that runs it.
    pub job: Option<i64>,
    /// `standard`, `fast` or `lite`.
    pub model: &'a str,
    /// `720p`, `1080p` or `4k`.
    pub resolution: &'a str,
    /// Seconds of finished video.
    pub seconds: u32,
    /// `16:9` or `9:16`.
    pub aspect: &'a str,
    /// The prompt handed to Google.
    pub prompt: &'a str,
    /// The brief's hash — the one its output is named after.
    pub brief_hash: &'a str,
    /// `scorsese_providers::prices::estimate`'s cents: the quoted figure.
    pub estimated_cents: u64,
}

/// The ledger's words for it.
pub(super) fn memo(shot: &Shot<'_>) -> String {
    format!(
        "Veo shot: {}s of {} at {}",
        shot.seconds, shot.model, shot.resolution
    )
}

/// The audit row of a shot.
pub(super) async fn insert(tx: &mut Tx, shot: &Shot<'_>, cost: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO veo_generations (user_id, project_id, tool_call_id, job_id, model,
             resolution, seconds, aspect, prompt, brief_hash, estimated_cost_micros)
         VALUES (member_id(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(shot.project)
    .bind(shot.tool_call)
    .bind(shot.job)
    .bind(shot.model)
    .bind(shot.resolution)
    .bind(i32::try_from(shot.seconds).unwrap_or(i32::MAX))
    .bind(shot.aspect)
    .bind(shot.prompt)
    .bind(shot.brief_hash)
    .bind(cost)
    .fetch_one(&mut **tx)
    .await
}
