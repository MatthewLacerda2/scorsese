//! A generated still's audit row (#461): `image_generations`, started and
//! settled by [`generations`](super::generations) like every paid
//! generation. A still is priced by its size and its references
//! (`scorsese_providers::prices::image`), at half that in a batch (#947),
//! whose row keeps the batch job's name as its ticket.

use crate::db::Tx;

/// A still about to be drawn (#461).
#[derive(Debug, Clone)]
pub struct Still<'a> {
    /// The project it is for.
    pub project: Option<i64>,
    /// The assistant tool call that asked for it.
    pub tool_call: Option<i64>,
    /// The job that runs it.
    pub job: Option<i64>,
    /// `nano_banana_2.1`, the only image model since #1308; older rows name
    /// the one they were drawn by.
    pub model: &'a str,
    /// `1K`, `2K` or `4K` (`0.5K` on older rows).
    pub resolution: &'a str,
    /// `16:9`, `1:1`, ...
    pub aspect: &'a str,
    /// How many reference pictures go with the prompt.
    pub references: usize,
    /// The prompt handed to Google.
    pub prompt: &'a str,
    /// The brief's hash — the one its output is named after.
    pub brief_hash: &'a str,
    /// `scorsese_providers::prices::image`'s cents: the quoted figure — or
    /// `image_in_batch`'s, half of it, when `batch` is set.
    pub estimated_cents: u64,
    /// Ordered in a half-price batch (#947) rather than drawn now.
    pub batch: bool,
}

/// The ledger's words for it.
pub(super) fn memo(still: &Still<'_>) -> String {
    let order = if still.batch { ", in a batch" } else { "" };
    format!(
        "Generated still: {} {} in {}{order}",
        still.resolution, still.aspect, still.model
    )
}

/// The audit row of a still.
pub(super) async fn insert(tx: &mut Tx, still: &Still<'_>, cost: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO image_generations (user_id, project_id, tool_call_id, job_id, model,
             resolution, aspect, references_sent, prompt, brief_hash, estimated_cost_micros,
             batch)
         VALUES (member_id(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(still.project)
    .bind(still.tool_call)
    .bind(still.job)
    .bind(still.model)
    .bind(still.resolution)
    .bind(still.aspect)
    .bind(i32::try_from(still.references).unwrap_or(i32::MAX))
    .bind(still.prompt)
    .bind(still.brief_hash)
    .bind(cost)
    .bind(still.batch)
    .fetch_one(&mut **tx)
    .await
}
