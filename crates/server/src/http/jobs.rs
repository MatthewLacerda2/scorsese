//! A user's jobs. Their changes arrive live on `GET /api/events`
//! ([`super::events`]); these are what a page reads first, and again on a
//! resync — and the one thing a user does to a job: stop it.

use axum::Json;
use axum::extract::{Path, State};

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::jobs::{CancelError, JobView, store};

/// `GET /api/jobs`: the caller's last hundred jobs, newest first, a running
/// render's progress folded in.
pub async fn list(
    State(state): State<AppState>,
    member: Member,
) -> Result<Json<Vec<JobView>>, ApiError> {
    let jobs = store::list(&state.pool, member.user).await?;
    Ok(Json(
        jobs.into_iter()
            .map(|job| state.jobs.progressed(job))
            .collect(),
    ))
}

/// `GET /api/jobs/{id}`: one of the caller's jobs. `404` for one that is not
/// theirs, whether or not it is somebody else's.
pub async fn get(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<Json<JobView>, ApiError> {
    store::get(&state.pool, member.user, id)
        .await?
        .map(|job| Json(state.jobs.progressed(job)))
        .ok_or(ApiError::NotFound)
}

/// `POST /api/jobs/{id}/cancel`: stop one of the caller's renders (#660). A
/// waiting one comes back `cancelled`; a running one comes back `running` and
/// turns `cancelled` on the event stream once it has stopped, within a frame;
/// a finished one comes back as it is. `404` for one that is not theirs, `409`
/// for a kind that is never stopped — a paid generation.
pub async fn cancel(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<Json<JobView>, ApiError> {
    match crate::jobs::cancel(&state.pool, &state.jobs, member.user, id).await {
        Ok(job) => Ok(Json(job)),
        Err(CancelError::NotFound) => Err(ApiError::NotFound),
        Err(refused @ CancelError::Unstoppable(_)) => Err(ApiError::Conflict(refused.to_string())),
        Err(CancelError::Database(error)) => Err(error.into()),
    }
}
