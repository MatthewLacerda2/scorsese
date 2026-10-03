//! A user's credits: their balance, and the history of what moved it
//! ([`crate::credits::history`]). Read-only — money arrives through the
//! operator (and later Pix), and leaves through the features that spend it.

use axum::Json;
use axum::extract::{Query, State};
use serde::Serialize;

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::credits::history::{self, Filter, History};
use crate::credits::{CreditError, ledger};
use crate::db;

/// What `GET /api/credits` answers.
#[derive(Debug, Serialize)]
pub struct Balance {
    /// The balance, in micro-dollars.
    pub balance_micros: i64,
}

/// `GET /api/credits`: the caller's balance, in dollars.
pub async fn balance(
    State(state): State<AppState>,
    member: Member,
) -> Result<Json<Balance>, ApiError> {
    let mut tx = db::scoped(&state.pool, member.user).await?;
    let balance = ledger::balance(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(Balance {
        balance_micros: balance,
    }))
}

/// `GET /api/credits/history?project=&kind=&since=&until=&before=&limit=`:
/// what moved the caller's balance, newest first, with the filter's total.
pub async fn history(
    State(state): State<AppState>,
    member: Member,
    Query(filter): Query<Filter>,
) -> Result<Json<History>, ApiError> {
    Ok(Json(
        history::read(&state.pool, member.user, &filter).await?,
    ))
}

impl From<CreditError> for ApiError {
    fn from(error: CreditError) -> Self {
        match error {
            CreditError::Database(error) => error.into(),
            CreditError::Unpriced(_) => {
                eprintln!("scorsese-server: {error}");
                Self::Internal
            }
            CreditError::Insufficient { .. }
            | CreditError::NotOpen(_)
            | CreditError::Invalid(_) => Self::BadRequest(error.to_string()),
        }
    }
}
