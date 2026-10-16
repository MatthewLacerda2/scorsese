//! A spoken line's audit row: `speech_generations`, started and settled by
//! [`generations`](super::generations) like every paid generation. A line is
//! priced by its characters (`scorsese_providers::prices::speech`), and the
//! row records the words and the voice.

use serde_json::Value;

use crate::db::Tx;

/// A line of narration about to be spoken.
#[derive(Debug, Clone)]
pub struct Line<'a> {
    /// The project it is for.
    pub project: Option<i64>,
    /// The assistant tool call that asked for it (#540).
    pub tool_call: Option<i64>,
    /// The job that runs it.
    pub job: Option<i64>,
    /// The model, as scorsese names it.
    pub model: &'a str,
    /// The voice id.
    pub voice: &'a str,
    /// What is spoken.
    pub text: &'a str,
    /// The rest of the request: stability, style, speed…
    pub settings: &'a Value,
    /// `scorsese_providers::prices::speech`'s cents: the quoted figure.
    pub estimated_cents: u64,
}

/// The ledger's words for it.
pub(super) fn memo(line: &Line<'_>) -> String {
    format!(
        "Spoken line: {} characters in {}",
        line.text.chars().count(),
        line.model
    )
}

/// The audit row of a line.
pub(super) async fn insert(tx: &mut Tx, line: &Line<'_>, cost: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO speech_generations (user_id, project_id, tool_call_id, job_id, model,
             voice, text, characters, settings, estimated_cost_micros)
         VALUES (member_id(), $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(line.project)
    .bind(line.tool_call)
    .bind(line.job)
    .bind(line.model)
    .bind(line.voice)
    .bind(line.text)
    .bind(i32::try_from(line.text.chars().count()).unwrap_or(i32::MAX))
    .bind(line.settings)
    .bind(cost)
    .fetch_one(&mut **tx)
    .await
}
