//! The assistant's routes (#540): what the web editor's chat panel (#545)
//! calls. The turn itself streams on `GET /api/events`; these start one, read
//! one back, stop one, and answer a quote or a question. [`crate::assistant`] has the
//! argument for each.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::json;

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::assistant::{
    self, Answer, Answered, Answering, AssistantError, Choice, Conversation, Opening, TurnDetail,
    TurnView,
};

/// `POST /api/projects/{id}/chat`'s body.
#[derive(Debug, Deserialize)]
pub struct Send {
    /// What the user wrote.
    pub prompt: String,
    /// Begin a new conversation about the project instead of continuing the
    /// newest one.
    #[serde(default)]
    pub fresh: bool,
    /// Library ids of the files attached to the message (#1534): brought
    /// into the project before the turn starts, and named to the model.
    #[serde(default)]
    pub attachments: Vec<i64>,
}

/// `POST /api/projects/{id}/chat`: a turn starts, `202` with it, at the
/// project's effort. `503` when the assistant is not configured, `402` when
/// the balance is empty, `409` while another turn of the conversation runs;
/// `400` when an attachment is not the caller's, `429` past the editor's
/// per-minute limit — and on any refusal nothing is imported.
pub async fn send(
    State(state): State<AppState>,
    member: Member,
    Path(project): Path<i64>,
    Json(send): Json<Send>,
) -> Result<(StatusCode, Json<TurnView>), ApiError> {
    let opening = Opening {
        prompt: send.prompt,
        fresh: send.fresh,
        attachments: send.attachments,
        ..Opening::default()
    };
    let turn = assistant::start(&state, member.user, project, opening).await?;
    Ok((StatusCode::ACCEPTED, Json(turn)))
}

/// `GET /api/projects/{id}/chat`: the project's current conversation.
pub async fn conversation(
    State(state): State<AppState>,
    member: Member,
    Path(project): Path<i64>,
) -> Result<Json<Conversation>, ApiError> {
    Ok(Json(
        assistant::conversation(&state, member.user, project).await?,
    ))
}

/// `PUT /api/projects/{id}/chat/model`'s body.
#[derive(Debug, Deserialize)]
pub struct ModelChoice {
    /// The id of the model the project's assistant runs on from now on.
    pub model: String,
}

/// `PUT /api/projects/{id}/chat/model`: the project's assistant runs on
/// another model from its next turn (#705). The model as the picker lists it;
/// `400` for a model the assistant does not offer.
pub async fn choose_model(
    State(state): State<AppState>,
    member: Member,
    Path(project): Path<i64>,
    Json(choice): Json<ModelChoice>,
) -> Result<Json<Choice>, ApiError> {
    Ok(Json(
        assistant::choose(&state, member.user, project, &choice.model).await?,
    ))
}

/// `PUT /api/projects/{id}/chat/effort`'s body.
#[derive(Debug, Deserialize)]
pub struct EffortChoice {
    /// The level the project's assistant runs at from now on: `low`,
    /// `medium`, `high` or `xhigh`.
    pub effort: String,
}

/// `PUT /api/projects/{id}/chat/effort`: the project's assistant runs at
/// another level from its next turn (#1443). `{effort}` as stored; `400` for
/// a level the assistant does not offer.
pub async fn choose_effort(
    State(state): State<AppState>,
    member: Member,
    Path(project): Path<i64>,
    Json(choice): Json<EffortChoice>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let effort = assistant::choose_effort(&state, member.user, project, &choice.effort).await?;
    Ok(Json(json!({ "effort": effort.as_str() })))
}

/// `GET /api/chat/turns/{id}`: one turn, and the log of its tool calls.
pub async fn turn(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<Json<TurnDetail>, ApiError> {
    Ok(Json(assistant::detail(&state.pool, member.user, id).await?))
}

/// `POST /api/chat/turns/{id}/stop`: ask a running turn to stop before its
/// next step. `202`; the turn's end arrives on the event stream. A turn paused
/// on a question stops at once, the question unanswered.
pub async fn stop(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let turn = assistant::detail(&state.pool, member.user, id).await?.turn;
    if turn.state == "asking" && assistant::set_aside(&state, member.user, id).await? {
        return Ok((StatusCode::ACCEPTED, Json(json!({ "stopping": id }))));
    }
    if turn.state != "running" {
        return Err(ApiError::Conflict(format!(
            "that turn is not running; it {}",
            turn.state
        )));
    }
    state.assistant.stop(id);
    Ok((StatusCode::ACCEPTED, Json(json!({ "stopping": id }))))
}

/// `POST /api/chat/turns/{id}/quote`'s body.
#[derive(Debug, Deserialize)]
pub struct QuoteAnswer {
    /// `true` spends; `false` withdraws the quote.
    pub confirm: bool,
    /// With `confirm: false`, the change the user wants to what was quoted
    /// (#709): nothing is spent, and the turn carries on with these words.
    #[serde(default)]
    pub change: Option<String>,
    /// With `confirm: true`, spend the half-price batch the quote offered
    /// beside itself instead (#947).
    #[serde(default)]
    pub batch: bool,
}

/// `POST /api/chat/turns/{id}/quote`: the user's answer to the quote a turn
/// is paused on, which resumes that turn with it (#1440). A yes spends; a
/// change has the assistant rewrite the briefs and quote again.
pub async fn quote(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(answer): Json<QuoteAnswer>,
) -> Result<Json<Answered>, ApiError> {
    let answer = match (answer.confirm, answer.change, answer.batch) {
        (true, Some(_), _) => {
            return Err(ApiError::BadRequest(
                "a yes spends what was quoted; ask for a change with confirm: false".into(),
            ));
        }
        (false, _, true) => {
            return Err(ApiError::BadRequest(
                "batch picks which yes; a no has nothing to pick".into(),
            ));
        }
        (true, None, batch) => Answer::Confirm { batch },
        (false, None, _) => Answer::Decline,
        (false, Some(change), _) => Answer::Change(change),
    };
    Ok(Json(
        assistant::answer_quote(&state, member.user, id, answer).await?,
    ))
}

/// `POST /api/chat/turns/{id}/answer`'s body.
#[derive(Debug, Deserialize)]
pub struct QuestionAnswer {
    /// The answer: one of the options' words, or the user's own. Beside a
    /// pick, a note on it, and it may be left out.
    #[serde(default)]
    pub answer: String,
    /// On a picker (#901), the `key`s of the candidates picked — `[]` for
    /// none of them. Left out, the words are the whole answer.
    #[serde(default)]
    pub picked: Option<Vec<String>>,
}

/// `POST /api/chat/turns/{id}/answer`: the user's answer to the question a
/// turn is paused on (#710), or their pick from its picker (#901). The same
/// turn resumes, `202` with it — what was picked is imported before the
/// model hears of it; `400` when no question waits there or the pick names a
/// candidate it did not offer, and refused like a new message when the
/// balance is empty or the model has no key.
pub async fn answer(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(answer): Json<QuestionAnswer>,
) -> Result<(StatusCode, Json<TurnView>), ApiError> {
    let answering = Answering {
        words: answer.answer,
        picked: answer.picked,
        notes: Vec::new(),
    };
    let turn = assistant::answer_question(&state, member.user, id, answering).await?;
    Ok((StatusCode::ACCEPTED, Json(turn)))
}

impl From<AssistantError> for ApiError {
    fn from(error: AssistantError) -> Self {
        let refused = |status: StatusCode| Self::Refused {
            status,
            message: error.to_string(),
            detail: json!({}),
        };
        match &error {
            AssistantError::NotConfigured(_) => refused(StatusCode::SERVICE_UNAVAILABLE),
            AssistantError::NoCredit(_) => refused(StatusCode::PAYMENT_REQUIRED),
            AssistantError::Busy => Self::Conflict(error.to_string()),
            AssistantError::TooMany(seconds) => Self::TooMany {
                message: error.to_string(),
                retry_after: *seconds,
            },
            AssistantError::NotFound => Self::NotFound,
            AssistantError::Invalid(why) => Self::BadRequest(why.clone()),
            AssistantError::Database(_) | AssistantError::Internal(_) => {
                eprintln!("scorsese-server: assistant: {error}");
                Self::Internal
            }
        }
    }
}
