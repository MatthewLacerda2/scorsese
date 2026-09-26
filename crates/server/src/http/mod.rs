//! The HTTP surface: the routes, and serving them until told to stop.
//!
//! | route | who | what |
//! | --- | --- | --- |
//! | `GET /api/health` | anyone | `200` when the database answers |
//! | `POST /api/login` | anyone | `{email, password}` → the account, and a session cookie |
//! | `POST /api/logout` | a member | ends the session |
//! | `GET /api/me` | a member | the account the request is logged in as |
//! | `POST /api/me/password` | a member | `{current, new}` |
//! | `GET /api/tokens` | a member | their API tokens, without values |
//! | `POST /api/tokens` | a member, by session | `{name}` → `{id, token}`, shown once |
//! | `DELETE /api/tokens/{id}` | a member | revokes one |
//!
//! "A member" is a request carrying a session cookie or an API token — see
//! [`auth`].

pub mod account;
pub mod auth;
pub mod error;
pub mod tokens;

use std::future::Future;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use sqlx::postgres::PgPool;
use tokio::net::TcpListener;

/// What every handler can reach.
///
/// Cheap to clone — a pool is a handle — which is what axum asks of state.
#[derive(Clone)]
pub struct AppState {
    /// The database, as [`db::member_pool`](crate::db::member_pool) makes
    /// it: every connection unable to read a table until a handler opens a
    /// scoped or privileged transaction.
    pub pool: PgPool,
}

/// Every route the server answers, all of them under `/api`.
///
/// One prefix for the whole API, health check included, so the web app's
/// dev proxy (`web/`, which forwards `/api` unchanged) and whatever fronts the
/// server in production route it with a single rule — and a path that is not
/// `/api/...` is never this server's, so it can go to the front-end.
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/login", post(account::login))
        .route("/logout", post(account::logout))
        .route("/me", get(account::me))
        .route("/me/password", post(account::change_password))
        .route("/tokens", get(tokens::list).post(tokens::issue))
        .route("/tokens/{id}", delete(tokens::revoke));
    Router::new().nest("/api", api).with_state(state)
}

/// Whether this server can do its job right now: `200 ok` or `503`.
///
/// It asks the database, because a server that is up and cannot reach its
/// database cannot serve a single real request — and the thing reading this
/// (the container's health check, the tunnel in front of it) needs to know
/// that, not merely that a process is listening.
async fn health(State(state): State<AppState>) -> (StatusCode, &'static str) {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "database unreachable"),
    }
}

/// Serve `router` on `listener` until `shutdown` completes.
///
/// A graceful stop: once `shutdown` fires, no new connection is accepted and
/// requests already in flight finish before this returns. `docker stop` sends
/// SIGTERM and waits before killing, and this is what makes that wait mean a
/// request is never cut off half-answered.
pub async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
}
