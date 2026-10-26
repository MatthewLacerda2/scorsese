//! When the generations a conversation ordered land, the assistant is woken
//! and carries on by itself (#1441): nobody has to write "so, how's it
//! going?".
//!
//! **What a turn waits on.** A quote's yes is the user's own call
//! (`quote`), and every job it queues names that call
//! ([`jobs::store::enqueue_by`](crate::jobs::store::enqueue_by)), so a turn's
//! ordered jobs are the jobs of its `user` calls. The yes marks the turn
//! `awaiting` ([`ordered`]); `told_job` remembers the newest job the model has
//! heard about, so a turn that orders twice is told of each job once.
//!
//! **When it wakes.** The waker ([`watch`]) listens to the event bus. When a
//! job reaches a final state — done, failed, cancelled, or stuck: given up on,
//! so the assistant should hear of it too — or a turn ends, it looks at that
//! conversation ([`check`]). A continuation starts only when the
//! conversation's **last** turn is the one awaiting, every job it waits on has
//! ended, and it ended by itself: a turn still running or asking is waited
//! out (one at a time; its end is the next look), and one the person stopped
//! is left alone. If the person wrote after the spend, their turn is the last
//! one, so nothing starts: the next message they send carries the note
//! instead ([`told`], called by `start` for every turn). A conversation a new
//! one set aside is never woken.
//!
//! **What it is.** An ordinary turn ([`super::start`] with
//! [`Opening::woken`](super::Opening)), on the conversation's model and the
//! project's effort: charged for its tokens, paused by a quote
//! like any other, and never spending on its own. The model is told what
//! landed in a server note, after a user message the server writes in words
//! never to be taken for the person's (`prompt::WOKEN`); the turn's stored
//! prompt, a line such as "Continued after the stills landed", is only for
//! the ledger, and the chat shows its own line in place of a bubble.
//!
//! **The person first.** A message and a woken start can race for the same
//! note: the message waits on the rows a woken start holds, a woken start
//! skips the rows a message holds, and only the start that clears `awaiting`
//! tells it.
//!
//! **Missed nudges.** The bus drops events for a reader that falls behind, and
//! a restart loses the ones in flight, so the waker also sweeps every awaiting
//! conversation when it starts and whenever it fell behind — the one
//! cross-user read here, and only to learn whose conversations to look at.

use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::watch as signal;

use super::{AssistantError, Opening};
use crate::db::{self, Tx, UserId};
use crate::events::Event;
use crate::http::AppState;
use crate::jobs::State;

/// The jobs of turn `$1`'s yes the model has not been told about, as a
/// literal so each statement built with it stays one static string.
macro_rules! untold {
    ($turn:literal) => {
        concat!(
            "SELECT j.id, j.kind, j.state, j.payload, j.error FROM jobs j
             JOIN tool_calls c ON c.id = j.tool_call_id
             WHERE c.client = 'user' AND c.turn_id = ",
            $turn,
            " AND j.id > (SELECT told_job FROM chat_turns WHERE id = ",
            $turn,
            ")"
        )
    };
}

/// Wake conversations as their work lands, until `stop` turns true or the
/// event bus closes.
pub async fn watch(state: AppState, mut stop: signal::Receiver<bool>) {
    let heard = state.events.every();
    let mut heard = std::pin::pin!(heard);
    sweep(&state).await;
    loop {
        let next = tokio::select! {
            _ = stop.wait_for(|stopping| *stopping) => return,
            next = heard.next() => next,
        };
        match next {
            None => return,
            Some(None) => sweep(&state).await,
            Some(Some((user, Event::Job(job)))) if is_final(job.state) => {
                landed(&state, user, job.id).await;
            }
            Some(Some((user, Event::ChatTurn { turn, .. }))) if is_over(&turn.state) => {
                check(&state, user, turn.id).await;
            }
            Some(Some(_)) => {}
        }
    }
}

/// Mark turn `turn` as awaiting the jobs its yes just ordered — if it ordered
/// any: a yes whose every item was already made queues nothing.
pub(super) async fn ordered(tx: &mut Tx, turn: i64) -> Result<(), sqlx::Error> {
    sqlx::query(concat!(
        "UPDATE chat_turns SET awaiting = true WHERE id = $1 AND EXISTS (",
        untold!("$1"),
        ")"
    ))
    .bind(turn)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// What a new turn of `session` tells the model about work that has landed
/// since it was ordered: one note, and the line a woken turn shows in place
/// of a prompt. Each turn it covers stops awaiting, so it is told once.
/// `None` when nothing has landed.
pub(super) async fn told(
    tx: &mut Tx,
    session: i64,
    woken: bool,
) -> Result<Option<(String, String)>, sqlx::Error> {
    // The person's message is preferred: it waits for a woken start that holds
    // these rows, while a woken start skips the rows a message holds and so
    // finds nothing to tell.
    let turns: Vec<i64> = sqlx::query_scalar(if woken {
        "SELECT id FROM chat_turns WHERE session_id = $1 AND awaiting ORDER BY id
         FOR UPDATE SKIP LOCKED"
    } else {
        "SELECT id FROM chat_turns WHERE session_id = $1 AND awaiting ORDER BY id FOR UPDATE"
    })
    .bind(session)
    .fetch_all(&mut **tx)
    .await?;
    let mut landed = Vec::new();
    for turn in turns {
        let jobs: Vec<Landed> = sqlx::query_as(concat!(untold!("$1"), " ORDER BY j.id"))
            .bind(turn)
            .fetch_all(&mut **tx)
            .await?;
        if jobs.iter().any(|job| !job.ended()) {
            continue;
        }
        let newest = jobs.iter().map(|job| job.id).max().unwrap_or(0);
        // `awaiting` again: a start that waited on the lock above finds the
        // turn told already, and tells nothing twice.
        let told = sqlx::query(
            "UPDATE chat_turns SET awaiting = false, told_job = greatest(told_job, $2)
             WHERE id = $1 AND awaiting",
        )
        .bind(turn)
        .bind(newest)
        .execute(&mut **tx)
        .await?;
        if told.rows_affected() == 1 {
            landed.extend(jobs);
        }
    }
    if landed.is_empty() {
        return Ok(None);
    }
    Ok(Some((note(&landed, woken), marker(&landed))))
}

/// A job a turn ordered, as the note tells of it.
#[derive(sqlx::FromRow)]
struct Landed {
    id: i64,
    kind: String,
    #[sqlx(try_from = "String")]
    state: State,
    payload: Value,
    error: Option<String>,
}

impl Landed {
    /// Whether it has stopped for good.
    fn ended(&self) -> bool {
        is_final(self.state)
    }

    /// What it was, in a word or two.
    fn noun(&self) -> &str {
        match self.kind.as_str() {
            "veo_shot" => "shot",
            "still_image" | "batch_still" => "still",
            "spoken_line" => "line",
            "music_track" => "music track",
            "voice_design" => "voice design",
            "voice_keep" => "kept voice",
            other => other,
        }
    }

    /// One line of the note: what it was, for which asset, and how it ended.
    fn line(&self) -> String {
        let what = match self.payload.get("asset").and_then(Value::as_str) {
            Some(asset) => format!("{} `{asset}`", self.noun()),
            None => format!("the {}", self.noun()),
        };
        let why = self.error.as_deref().unwrap_or("no reason was given");
        match self.state {
            State::Done => format!("- {what}: made, and in the project or the library."),
            State::Cancelled => format!("- {what}: cancelled, so nothing was charged."),
            State::Stuck => format!("- {what}: the provider took too long, so it was given up on."),
            _ => format!("- {what}: failed, and was not charged. {why}"),
        }
    }
}

/// The server's note on what landed.
fn note(landed: &[Landed], woken: bool) -> String {
    let lines: Vec<String> = landed.iter().map(Landed::line).collect();
    let opening = if woken {
        "The person has not written. The server started this turn because the work ordered \
         earlier in this conversation has finished:"
    } else {
        "The work ordered earlier in this conversation has finished:"
    };
    let next = if woken {
        "Carry on with the plan you agreed with the person. If nothing is left to do, say in a \
         line what landed; if something failed, say so plainly and what you suggest."
    } else {
        "Their message follows; take this into account."
    };
    format!("{opening}\n{}\n{next}", lines.join("\n"))
}

/// The line a woken turn shows in place of a prompt: "Continued after the
/// stills landed".
fn marker(landed: &[Landed]) -> String {
    let first = landed.first().map(Landed::noun).unwrap_or("generation");
    let what = if landed.iter().all(|job| job.noun() == first) {
        if landed.len() == 1 {
            format!("the {first}")
        } else {
            format!("the {first}s")
        }
    } else {
        "the generations".to_owned()
    };
    format!("Continued after {what} landed")
}

/// Whether a job in `state` has stopped for good.
fn is_final(state: State) -> bool {
    !matches!(state, State::Waiting | State::Running)
}

/// Whether a turn in `state` has ended.
fn is_over(state: &str) -> bool {
    !matches!(state, "running" | "asking")
}

/// Job `job` of `user`'s ended: look at the conversation of the turn that
/// ordered it, if a turn did.
async fn landed(state: &AppState, user: UserId, job: i64) {
    let turn = async {
        let mut tx = db::scoped(&state.pool, user).await?;
        let turn: Option<i64> = sqlx::query_scalar(
            "SELECT c.turn_id FROM jobs j JOIN tool_calls c ON c.id = j.tool_call_id
             WHERE j.id = $1 AND c.client = 'user'",
        )
        .bind(job)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        tx.commit().await?;
        Ok::<_, sqlx::Error>(turn)
    };
    match turn.await {
        Ok(Some(turn)) => check(state, user, turn).await,
        Ok(None) => {}
        Err(error) => tracing::error!("assistant: job {job} landed: {error}"),
    }
}

/// Look at the conversation turn `turn` of `user`'s is in, and start a
/// continuation there if its last turn's work has all landed.
async fn check(state: &AppState, user: UserId, turn: i64) {
    match due(state, user, turn).await {
        Ok(Some(project)) => {
            // `effort: None`: the project's level, as for a message.
            let opening = Opening {
                woken: true,
                ..Opening::default()
            };
            match super::start(state, user, project, opening).await {
                // Somebody got there first, or the work was told meanwhile.
                Ok(_) | Err(AssistantError::Busy | AssistantError::Invalid(_)) => {}
                // An empty balance or a missing key: the note waits for the
                // person's next message.
                Err(error) => tracing::error!("assistant: waking turn {turn}: {error}"),
            }
        }
        Ok(None) => {}
        Err(error) => tracing::error!("assistant: waking turn {turn}: {error}"),
    }
}

/// Whether turn `turn`'s conversation is due a continuation: its project,
/// if so.
async fn due(state: &AppState, user: UserId, turn: i64) -> Result<Option<i64>, sqlx::Error> {
    let mut tx = db::scoped(&state.pool, user).await?;
    // A conversation a newer one set aside is never woken: it stops waiting.
    sqlx::query(
        "UPDATE chat_turns t SET awaiting = false FROM chat_sessions s
         WHERE t.awaiting AND s.id = t.session_id
           AND t.session_id = (SELECT session_id FROM chat_turns WHERE id = $1)
           AND s.id <> (SELECT max(id) FROM chat_sessions WHERE project_id = s.project_id)",
    )
    .bind(turn)
    .execute(&mut *tx)
    .await?;
    let last: Option<(i64, i64, String, bool)> = sqlx::query_as(
        "SELECT t.id, s.project_id, t.state, t.awaiting
         FROM chat_turns t JOIN chat_sessions s ON s.id = t.session_id
         WHERE t.session_id = (SELECT session_id FROM chat_turns WHERE id = $1)
           AND s.id = (SELECT max(id) FROM chat_sessions WHERE project_id = s.project_id)
         ORDER BY t.id DESC LIMIT 1",
    )
    .bind(turn)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((last, project, ended, awaiting)) = last else {
        tx.commit().await?;
        return Ok(None);
    };
    if !awaiting || !is_over(&ended) || ended == "stopped" {
        tx.commit().await?;
        return Ok(None);
    }
    let pending: bool = sqlx::query_scalar(concat!(
        "SELECT EXISTS (",
        untold!("$1"),
        " AND j.state IN ('waiting', 'running'))"
    ))
    .bind(last)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((!pending).then_some(project))
}

/// Look at every conversation with a turn awaiting work, whoever's.
async fn sweep(state: &AppState) {
    let awaiting = async {
        let mut tx = db::privileged(&state.pool).await?;
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT DISTINCT ON (session_id) user_id, id FROM chat_turns WHERE awaiting
             ORDER BY session_id, id DESC",
        )
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(rows)
    };
    match awaiting.await {
        Ok(rows) => {
            for (user, turn) in rows {
                check(state, UserId::from_row(user), turn).await;
            }
        }
        Err(error) => tracing::error!("assistant: sweeping for landed work: {error}"),
    }
}
