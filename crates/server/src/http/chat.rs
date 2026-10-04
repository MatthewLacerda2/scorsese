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
    self, Answer, Answered, AssistantError, Choice, Conversation, Opening, TurnDetail, TurnView,
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
}

/// `POST /api/projects/{id}/chat`: a turn starts, `202` with it. `503` when
/// the assistant is not configured, `402` when the balance is empty, `409`
/// while another turn of the conversation runs.
pub async fn send(
    State(state): State<AppState>,
    member: Member,
    Path(project): Path<i64>,
    Json(send): Json<Send>,
) -> Result<(StatusCode, Json<TurnView>), ApiError> {
    let opening = Opening {
        prompt: send.prompt,
        fresh: send.fresh,
        notes: Vec::new(),
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
    /// (#709): nothing is spent, and a turn starts with these words.
    #[serde(default)]
    pub change: Option<String>,
}

/// `POST /api/chat/turns/{id}/quote`: the user's answer to the quote held on
/// a turn. A yes spends and starts a turn to carry on; a change starts one
/// that rewrites the briefs and quotes again.
pub async fn quote(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(answer): Json<QuoteAnswer>,
) -> Result<Json<Answered>, ApiError> {
    let answer = match (answer.confirm, answer.change) {
        (true, Some(_)) => {
            return Err(ApiError::BadRequest(
                "a yes spends what was quoted; ask for a change with confirm: false".into(),
            ));
        }
        (true, None) => Answer::Confirm,
        (false, None) => Answer::Decline,
        (false, Some(change)) => Answer::Change(change),
    };
    Ok(Json(
        assistant::answer_quote(&state, member.user, id, answer).await?,
    ))
}

/// `POST /api/chat/turns/{id}/answer`'s body.
#[derive(Debug, Deserialize)]
pub struct QuestionAnswer {
    /// The answer: one of the options' words, or the user's own.
    pub answer: String,
}

/// `POST /api/chat/turns/{id}/answer`: the user's answer to the question a
/// turn is paused on (#710). The same turn resumes, `202` with it; `400`
/// when no question waits there, and refused like a new message when the
/// balance is empty or the model has no key.
pub async fn answer(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(answer): Json<QuestionAnswer>,
) -> Result<(StatusCode, Json<TurnView>), ApiError> {
    let turn = assistant::answer_question(&state, member.user, id, &answer.answer).await?;
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
            AssistantError::NotFound => Self::NotFound,
            AssistantError::Invalid(why) => Self::BadRequest(why.clone()),
            AssistantError::Database(_) | AssistantError::Internal(_) => {
                eprintln!("scorsese-server: assistant: {error}");
                Self::Internal
            }
        }
    }
}
