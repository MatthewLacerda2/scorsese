//! Buying credits (#548): the quote a person sees as they type, the Checkout
//! Session they are sent to, the return page's confirmation, and Stripe's
//! webhook. [`crate::payments`] has the flow and why it is shaped so.

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use scorsese_providers::payments::webhook::{self, Happened};
use serde::Deserialize;
use serde_json::json;

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::payments::settle::{self, Payment, Settled};
use crate::payments::{Begun, PaymentError, Payments, Quote, begin};

/// An amount of reais, in centavos.
#[derive(Debug, Deserialize)]
pub struct Amount {
    /// The reais, in centavos: `5000` is R$ 50,00.
    pub centavos: i64,
}

/// A purchase: the reais, and the rate the person was shown for them.
#[derive(Debug, Deserialize)]
pub struct Purchase {
    /// The reais, in centavos.
    pub centavos: i64,
    /// The quote's `brl_per_usd` the person saw. When it is no longer the
    /// rate, the purchase is refused with `409` and the quote now.
    pub brl_per_usd: Option<String>,
}

/// `GET /api/payments/quote?centavos=`: what that many reais buy now.
pub async fn quote(
    State(state): State<AppState>,
    _member: Member,
    Query(amount): Query<Amount>,
) -> Result<Json<Quote>, ApiError> {
    Ok(Json(configured(&state)?.quote(amount.centavos).await?))
}

/// `POST /api/payments` with `{"centavos", "brl_per_usd"}`: start a
/// purchase at the rate the person was shown, and answer with Stripe's page
/// to send the browser to.
pub async fn start(
    State(state): State<AppState>,
    member: Member,
    Json(purchase): Json<Purchase>,
) -> Result<Json<Begun>, ApiError> {
    let payments = configured(&state)?;
    let shown = purchase.brl_per_usd.as_deref();
    Ok(Json(
        begin(&state.pool, payments, member.user, purchase.centavos, shown).await?,
    ))
}

/// `POST /api/payments/{session}/confirm`: the caller's payment, credited
/// first if Stripe says it is paid and the webhook has not said so yet.
pub async fn confirm(
    State(state): State<AppState>,
    member: Member,
    Path(session): Path<String>,
) -> Result<Json<Payment>, ApiError> {
    let payments = configured(&state)?;
    Ok(Json(
        settle::confirm(&state.pool, payments, member.user, &session).await?,
    ))
}

/// `POST /api/payments/stripe/webhook`: Stripe telling us how a session went.
///
/// Public — Stripe has no session cookie — and so believed only when the
/// `Stripe-Signature` header signs the raw body. Answered `200` for anything
/// signed that needs nothing done, so Stripe stops retrying it; a failure on
/// our side is a `500`, which Stripe retries.
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let payments = configured(&state)?;
    let signature = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok());
    let event = webhook::verify(
        signature,
        &body,
        &payments.webhook_secret,
        scorsese_core::Timestamp::unix_now_or_epoch(),
    )
    .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let happened = Happened::of(&event).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let session = match &happened {
        Happened::Settled(session) | Happened::Expired(session) => session,
        Happened::Other => return Ok(Json(json!({ "received": event.kind }))),
    };
    let settled = settle::settle(&state.pool, session).await?;
    if settled == Settled::Unknown {
        tracing::warn!(
            "Stripe event {} names session {}, which is no payment here",
            event.id,
            session.id
        );
    }
    Ok(Json(json!({ "received": event.kind })))
}

/// The server's Stripe, or `503` where none is configured.
fn configured(state: &AppState) -> Result<&Payments, ApiError> {
    state.payments.as_ref().ok_or_else(|| ApiError::Refused {
        status: StatusCode::SERVICE_UNAVAILABLE,
        message: "buying credits is not configured on this server".to_owned(),
        detail: json!({}),
    })
}

/// Seconds since the epoch, for a signature's timestamp.
impl From<PaymentError> for ApiError {
    fn from(error: PaymentError) -> Self {
        let refused = |status| Self::Refused {
            status,
            message: error.to_string(),
            detail: json!({}),
        };
        match &error {
            PaymentError::OutOfRange => Self::BadRequest(error.to_string()),
            PaymentError::NotFound => Self::NotFound,
            PaymentError::NoRate(why) => {
                tracing::warn!("no rate for a purchase: {why}");
                refused(StatusCode::SERVICE_UNAVAILABLE)
            }
            PaymentError::StaleRate => {
                tracing::warn!("a purchase was refused: {error}");
                refused(StatusCode::SERVICE_UNAVAILABLE)
            }
            PaymentError::RateMoved(quote) => Self::Refused {
                status: StatusCode::CONFLICT,
                message: error.to_string(),
                detail: json!({ "quote": quote }),
            },
            PaymentError::Stripe(_) | PaymentError::NoPage(_) => {
                tracing::warn!("{error}");
                refused(StatusCode::BAD_GATEWAY)
            }
            PaymentError::Interrupted | PaymentError::Credit(_) | PaymentError::Database(_) => {
                tracing::error!("a payment failed: {error}");
                Self::Internal
            }
        }
    }
}
