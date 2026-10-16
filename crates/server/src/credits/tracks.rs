//! A music track's audit row (#1264): `music_generations`, the fifth kind
//! of paid generation beside a shot, a still, a line and a design.
//!
//! Started and settled by [`generations`](super::generations) like the
//! others — reserved before ElevenLabs is asked, charged once it answers,
//! free when it refuses. A track is priced by the seconds it asks for
//! (`scorsese_providers::prices::music`), and the row records them.

use crate::db::Tx;

/// A track of music about to be composed (#1264).
#[derive(Debug, Clone)]
pub struct Track<'a> {
    /// The project it is for.
    pub project: Option<i64>,
    /// The assistant tool call that asked for it.
    pub tool_call: Option<i64>,
    /// The job that runs it.
    pub job: Option<i64>,
    /// The vendor's model id.
    pub model: &'a str,
    /// The description of the music.
    pub prompt: &'a str,
    /// How long it was asked to be.
    pub seconds: u32,
    /// Whether it is guaranteed to have no singing.
    pub instrumental: bool,
    /// The brief's hash — the one its output is named after.
    pub brief_hash: &'a str,
    /// `scorsese_providers::prices::music`'s cents: the quoted figure.
    pub estimated_cents: u64,
}

/// The ledger's words for it.
pub(super) fn memo(track: &Track<'_>) -> String {
    format!("Music track: {}s in {}", track.seconds, track.model)
}

/// The audit row of a track.
pub(super) async fn insert(tx: &mut Tx, track: &Track<'_>, cost: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO music_generations (user_id, project_id, tool_call_id, job_id, model,
             prompt, seconds, instrumental, brief_hash, estimated_cost_micros)
         VALUES (member_id(), $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(track.project)
    .bind(track.tool_call)
    .bind(track.job)
    .bind(track.model)
    .bind(track.prompt)
    .bind(i32::try_from(track.seconds).unwrap_or(i32::MAX))
    .bind(track.instrumental)
    .bind(track.brief_hash)
    .bind(cost)
    .fetch_one(&mut **tx)
    .await
}
