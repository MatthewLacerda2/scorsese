//! `GET /api/prices` (#1538): what the public pricing page shows, from the
//! tables that charge — the examples ([`crate::credits::examples`]), the free
//! allowance this server gives, and the recharge cards at the rate kept now
//! ([`crate::payments::Payments::recharge`]). Public: a visitor reads it
//! before signing in, and nothing in it is anyone's.

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use super::AppState;
use crate::credits::examples::{self, Example};
use crate::payments::Recharge;

/// Everything the pricing page shows.
#[derive(Debug, Serialize)]
pub struct Prices {
    /// What things cost a user, in the page's order.
    pub examples: Vec<Example>,
    /// What every account is given free.
    pub free: Free,
    /// The recharge cards, or `None` on a server that sells nothing.
    pub recharge: Option<Recharge>,
}

/// The free allowance, in micro-dollars.
#[derive(Debug, Serialize)]
pub struct Free {
    /// The most a Brasília day may take from it.
    pub daily_micros: i64,
    /// The most a calendar month may take from it.
    pub monthly_micros: i64,
}

/// `GET /api/prices`.
pub async fn prices(State(state): State<AppState>) -> Json<Prices> {
    Json(Prices {
        examples: examples::examples(),
        free: Free {
            daily_micros: state.free.daily_micros,
            monthly_micros: state.free.monthly_micros,
        },
        recharge: state.payments.as_ref().map(|payments| payments.recharge()),
    })
}
