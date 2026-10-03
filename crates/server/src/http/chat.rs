//! The assistant's routes (#540): what the web editor's chat panel (#545)
//! calls. The turn itself streams on `GET /api/events`; these start one, read
//! one back, stop one, and answer a quote. [`crate::assistant`] has the
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
    self, Answered, AssistantError, Choice, Conversation, Opening, TurnDetail, TurnView,
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
/// next step. `202`; the turn's end arrives on the event stream.
pub async fn stop(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let turn = assistant::detail(&state.pool, member.user, id).await?.turn;
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
}

/// `POST /api/chat/turns/{id}/quote`: the user's answer to the quote held on
/// a turn. A yes spends and starts a turn to carry on.
pub async fn quote(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(answer): Json<QuoteAnswer>,
) -> Result<Json<Answered>, ApiError> {
    Ok(Json(
        assistant::answer_quote(&state, member.user, id, answer.confirm).await?,
    ))
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
