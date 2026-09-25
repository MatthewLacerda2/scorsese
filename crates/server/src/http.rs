//! The HTTP surface: the routes, and serving them until told to stop.

use std::future::Future;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use sqlx::postgres::PgPool;
use tokio::net::TcpListener;

/// What every handler can reach.
///
/// Cheap to clone — a pool is a handle — which is what axum asks of state.
#[derive(Clone)]
pub struct AppState {
    /// The database.
    pub pool: PgPool,
}

/// Every route the server answers, all of them under `/api`.
///
/// One prefix for the whole API, health check included, so the web app's
/// dev proxy (`web/`, which forwards `/api` unchanged) and whatever fronts the
/// server in production route it with a single rule — and a path that is not
/// `/api/...` is never this server's, so it can go to the front-end.
pub fn router(state: AppState) -> Router {
    let api = Router::new().route("/health", get(health));
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
