//! A user's jobs. Their changes arrive live on `GET /api/events`
//! ([`super::events`]); these are what a page reads first, and again on a
//! resync.

use axum::Json;
use axum::extract::{Path, State};

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::jobs::{JobView, store};

/// `GET /api/jobs`: the caller's last hundred jobs, newest first.
pub async fn list(
    State(state): State<AppState>,
    member: Member,
) -> Result<Json<Vec<JobView>>, ApiError> {
    Ok(Json(store::list(&state.pool, member.user).await?))
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
        .map(Json)
        .ok_or(ApiError::NotFound)
}
