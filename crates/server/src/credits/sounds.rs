//! A sound effect's audit row (#1265): `sound_generations`, the sixth kind
//! of paid generation beside a shot, a still, a line, a design and a track.
//!
//! Started and settled by [`generations`](super::generations) like the
//! others — reserved before ElevenLabs is asked, charged once it answers,
//! free when it refuses. A sound is priced by the milliseconds it asks for
//! (`scorsese_providers::prices::sound`), and the row records them.

use crate::db::Tx;

/// A sound effect about to be made (#1265).
#[derive(Debug, Clone)]
pub struct Sound<'a> {
    /// The project it is for.
    pub project: Option<i64>,
    /// The assistant tool call that asked for it.
    pub tool_call: Option<i64>,
    /// The job that runs it.
    pub job: Option<i64>,
    /// The vendor's model id.
    pub model: &'a str,
    /// The description of the sound.
    pub prompt: &'a str,
    /// How long it was asked to be, in milliseconds.
    pub millis: u32,
    /// Whether it is made to loop seamlessly.
    pub looping: bool,
    /// The brief's hash — the one its output is named after.
    pub brief_hash: &'a str,
    /// `scorsese_providers::prices::sound`'s cents: the quoted figure.
    pub estimated_cents: u64,
}

/// The ledger's words for it: `Sound effect: 2.5s in …`, with ` (loop)` for
/// one made to loop.
pub(super) fn memo(sound: &Sound<'_>) -> String {
    let seconds = f64::from(sound.millis) / 1000.0;
    let looping = if sound.looping { " (loop)" } else { "" };
    format!("Sound effect: {seconds}s in {}{looping}", sound.model)
}

/// The audit row of a sound.
pub(super) async fn insert(tx: &mut Tx, sound: &Sound<'_>, cost: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO sound_generations (user_id, project_id, tool_call_id, job_id, model,
             prompt, millis, loop, brief_hash, estimated_cost_micros)
         VALUES (member_id(), $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(sound.project)
    .bind(sound.tool_call)
    .bind(sound.job)
    .bind(sound.model)
    .bind(sound.prompt)
    .bind(i32::try_from(sound.millis).unwrap_or(i32::MAX))
    .bind(sound.looping)
    .bind(sound.brief_hash)
    .bind(cost)
    .fetch_one(&mut **tx)
    .await
}

#[cfg(test)]
mod tests {
    use super::{Sound, memo};

    fn sound(millis: u32, looping: bool) -> Sound<'static> {
        Sound {
            project: None,
            tool_call: None,
            job: None,
            model: "eleven_text_to_sound_v3",
            prompt: "a door slams",
            millis,
            looping,
            brief_hash: "h",
            estimated_cents: 1,
        }
    }

    /// The history says how long, in seconds a person reads, and whether it
    /// loops.
    #[test]
    fn the_memo_reads_in_seconds() {
        assert_eq!(
            memo(&sound(2500, false)),
            "Sound effect: 2.5s in eleven_text_to_sound_v3"
        );
        assert_eq!(
            memo(&sound(4000, true)),
            "Sound effect: 4s in eleven_text_to_sound_v3 (loop)"
        );
    }
}
