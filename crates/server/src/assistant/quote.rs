//! The user's answer to a quote the assistant showed them.
//!
//! **Yes** spends: the server calls the paid tool itself with the held token,
//! recorded as the user's own call (`client = 'user'`), and starts a turn
//! that tells the model — as a `system` message, which it can trust — what the
//! spend did. **No** withdraws the token, and the next turn is told. Either
//! way the token is claimed from the turn in one statement first, so two
//! clicks cannot answer one quote twice.

use serde::Serialize;
use serde_json::json;

use super::AssistantError;
use super::start::{Opening, start};
use super::store::{QuoteView, TurnView};
use crate::db::{self, UserId};
use crate::http::AppState;
use crate::tools::Client;

/// What answering a quote did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Answered {
    /// What the paid tool said when it spent — `null` for a no.
    pub spent: Option<String>,
    /// Whether the paid tool refused (an expired quote, a brief changed since,
    /// a balance that no longer covers it).
    pub refused: bool,
    /// The turn started to carry on after a yes — `null` for a no, or when no
    /// turn could start (the balance ran out on the spend itself).
    pub turn: Option<TurnView>,
    /// Why no turn started, when one did not.
    pub note: Option<String>,
}

/// Answer the quote held on `user`'s turn `id`: spend it on `yes`, withdraw it
/// otherwise.
pub async fn answer(
    state: &AppState,
    user: UserId,
    id: i64,
    yes: bool,
) -> Result<Answered, AssistantError> {
    let mut tx = db::scoped(&state.pool, user).await?;
    let busy: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM chat_turns r WHERE r.session_id = t.session_id
                                                   AND r.state = 'running')
         FROM chat_turns t WHERE t.id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    match busy {
        None => return Err(AssistantError::NotFound),
        Some(true) => return Err(AssistantError::Busy),
        Some(false) => {}
    }
    let claimed: Option<(String, sqlx::types::Json<QuoteView>, i64)> = sqlx::query_as(
        "WITH held AS (
             SELECT id, quote_token FROM chat_turns
             WHERE id = $1 AND quote_token IS NOT NULL FOR UPDATE)
         UPDATE chat_turns t SET quote_token = NULL, quote_answer = $2
         FROM held, chat_sessions s
         WHERE t.id = held.id AND s.id = t.session_id
         RETURNING held.quote_token, t.quote, s.project_id",
    )
    .bind(id)
    .bind(if yes { "confirmed" } else { "declined" })
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let Some((token, quote, project)) = claimed else {
        return Err(AssistantError::Invalid(
            "there is no quote waiting for an answer on that turn".into(),
        ));
    };
    if !yes {
        state.tools.withdraw_quote(user, &token).await?;
        return Ok(Answered {
            spent: None,
            refused: false,
            turn: None,
            note: None,
        });
    }

    let arguments = json!({ "project": project, "confirm": token });
    let client = Client::User { turn: id };
    let outcome = state
        .tools
        .call(user, client, &quote.tool, &arguments)
        .await;
    let (said, refused) = match outcome {
        Ok(reply) => {
            let words: Vec<String> = reply.parts.into_iter().map(|part| part.text).collect();
            (words.join("\n"), false)
        }
        Err(refusal) => (refusal, true),
    };
    let note = if refused {
        format!(
            "The person said yes to the quote, but {} refused to spend: {said}",
            quote.tool
        )
    } else {
        format!(
            "The person confirmed the quote. {} was called with it and answered:\n{said}",
            quote.tool
        )
    };
    let opening = Opening {
        prompt: "Yes, go ahead.".into(),
        fresh: false,
        notes: vec![note],
    };
    let (turn, note) = match start(state, user, project, opening).await {
        Ok(turn) => (Some(turn), None),
        Err(error) => (None, Some(error.to_string())),
    };
    Ok(Answered {
        spent: Some(said),
        refused,
        turn,
        note,
    })
}
