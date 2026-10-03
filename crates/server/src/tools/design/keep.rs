//! What `voice_design` answers without spending: a design the user already
//! has, keeping one of its candidates, and the voices they designed.

use scorsese_mcp::Reply;

use super::super::{Caller, database};
use crate::credits::dollars;
use crate::db;
use crate::designs::{self, KeepPayload};
use crate::jobs::{kinds, store as jobs};

/// What to say instead of quoting, when the design from brief `digest` costs
/// nothing: the user has it, every sample in their library, or a job of
/// theirs is designing it now.
pub(super) async fn already(caller: &Caller<'_>, digest: &str) -> Result<Option<String>, String> {
    let toolbox = caller.toolbox;
    let mut tx = db::scoped(&toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let design = designs::find(&mut tx, digest).await.map_err(database)?;
    let running: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM jobs WHERE kind = $1 AND state IN ('waiting', 'running')
           AND payload->>'brief' = $2 ORDER BY id LIMIT 1",
    )
    .bind(kinds::VOICE_DESIGN.name)
    .bind(digest)
    .fetch_optional(&mut *tx)
    .await
    .map_err(database)?;
    tx.commit().await.map_err(database)?;
    if let Some(design) = design
        && let Some(items) = designs::samples(&toolbox.library, caller.user, &design)
            .await
            .map_err(database)?
    {
        let mut lines: Vec<String> = design
            .candidates
            .iter()
            .zip(&items)
            .enumerate()
            .map(|(index, (sample, item))| {
                format!(
                    "{}. {}\n   library item {} ({})",
                    index + 1,
                    sample.generated_voice_id,
                    item.id,
                    item.name
                )
            })
            .collect();
        lines.push(String::new());
        lines.push(
            "Already designed — these three were paid for before and their samples are in your \
             library, so this cost nothing. Play a sample in the library, or import it into a \
             project and hear it, to choose. Then call voice_design with keep set to that \
             candidate's id and a name for it."
                .to_owned(),
        );
        return Ok(Some(lines.join("\n")));
    }
    Ok(running.map(|job| {
        format!(
            "This design is already on its way as job {job} — paid for, nothing more to pay. \
             Call jobs with job: {job} to see it finish, then call this again for the \
             candidates."
        )
    }))
}

/// Keep candidate `chosen` of one of the caller's designs as a voice called
/// `name`: queued, free.
pub(super) async fn keep(caller: &Caller<'_>, chosen: &str, name: &str) -> Result<Reply, String> {
    let toolbox = caller.toolbox;
    let mut tx = db::scoped(&toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let Some(design) = designs::offering(&mut tx, chosen).await.map_err(database)? else {
        return Err(format!(
            "none of your designs offered a candidate `{chosen}`; design one with prompt and \
             text, and keep one of the ids it answers with"
        ));
    };
    let payload = KeepPayload {
        design: design.id,
        chosen: chosen.to_owned(),
        name: name.to_owned(),
    };
    let payload = serde_json::to_value(&payload).map_err(database)?;
    let job = jobs::enqueue(&mut tx, kinds::VOICE_KEEP, &payload)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    toolbox.queue.announce(caller.user, &job);
    Ok(format!(
        "Keeping {chosen} as \"{name}\", as job {} — free. Call jobs with job: {}; its result \
         is the voice_id a narration names. The voice lives in scorsese's ElevenLabs account, \
         not in any project, so it is yours in every project; the description and seed that \
         made it are kept with it, and voice_design with list shows them.",
        job.id, job.id
    )
    .into())
}

/// Every voice the caller designed and kept, and what designing them came to.
pub(super) async fn list(caller: &Caller<'_>) -> Result<Reply, String> {
    let mut tx = db::scoped(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let kept = designs::designed(&mut tx).await.map_err(database)?;
    tx.commit().await.map_err(database)?;
    if kept.is_empty() {
        return Ok(
            "You have not designed and kept any voices yet. Call voice_design with \
                   prompt and text to design three candidates to choose between."
                .into(),
        );
    }
    let mut lines: Vec<String> = kept
        .iter()
        .map(|voice| {
            let seed = voice
                .seed
                .map_or_else(|| "no seed".to_owned(), |seed| format!("seed {seed}"));
            format!(
                "{}  {}  ({seed})\n    {}",
                voice.voice_id, voice.name, voice.prompt
            )
        })
        .collect();
    // Once per design, as locally: keeping a second candidate of the same
    // three spent nothing more.
    let mut designs: Vec<(i64, i64)> = kept
        .iter()
        .map(|voice| (voice.design, voice.estimated_cost_micros))
        .collect();
    designs.sort_unstable();
    designs.dedup();
    let spent: i64 = designs.iter().map(|(_, micros)| micros).sum();
    lines.push(format!(
        "\n{} designed and kept, about {} at ElevenLabs' published rate spent designing them — \
         our arithmetic, never a bill. What each design took from your credits is in \
         spending_history.",
        kept.len(),
        dollars(spent)
    ));
    Ok(lines.join("\n").into())
}
