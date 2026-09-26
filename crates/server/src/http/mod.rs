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
//! | `GET /api/jobs` | a member | their last hundred jobs, newest first |
//! | `GET /api/jobs/{id}` | a member | one of their jobs |
//! | `GET /api/events` | a member | their live updates, as server-sent events |
//! | `GET /api/projects` | a member | their projects, without documents |
//! | `POST /api/projects` | a member | `{name, fps?}` → a new empty project |
//! | `GET /api/projects/{id}` | a member | the project, its document and revision |
//! | `PUT /api/projects/{id}` | a member | `{revision, document}`; `409` if it moved on |
//! | `PATCH /api/projects/{id}` | a member | `{name}`: rename |
//! | `DELETE /api/projects/{id}` | a member | deletes one |
//! | `GET /api/credits` | a member | their balance, in dollars and ≈ reais |
//! | `GET /api/credits/history` | a member | what moved it, filterable, with a total |
//! | `GET /api/library` | a member | their files: `?kind=&search=&sha256=` |
//! | `GET /api/library/{id}` | a member | one file's details, and the projects using it |
//! | `PATCH /api/library/{id}` | a member | `{name?, description?}` |
//! | `DELETE /api/library/{id}` | a member | `409` naming the projects that use it |
//! | `GET /api/library/{id}/file` | a member | the file; video and audio in ranges |
//! | `GET /api/library/{id}/thumbnail` | a member | its thumbnail, or `404` while it is drawn |
//! | `OPTIONS`, `POST /api/uploads` | a member | tus: what is supported; announce an upload |
//! | `HEAD`, `PATCH`, `DELETE /api/uploads/{id}` | a member | tus: how far; the next chunk; abandon |
//! | `POST /api/projects/{id}/renders` | a member | `{container?, video_codec?, audio_codec?, resolution?}` → the kept render (`200`) or its job (`202`) |
//! | `GET /api/projects/{id}/renders` | a member | the project's kept renders |
//! | `GET /api/renders/{id}/file` | a member | the file, in ranges; counts as use |
//!
//! "A member" is a request carrying a session cookie or an API token — see
//! [`auth`].

pub mod account;
pub mod auth;
pub mod credits;
pub mod error;
pub mod events;
pub mod jobs;
pub mod library;
pub mod projects;
mod ranges;
pub mod renders;
pub mod tokens;
pub mod uploads;

use std::future::Future;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{delete, get, head, post};
use sqlx::postgres::PgPool;
use tokio::net::TcpListener;

use crate::Files;
use crate::events::Events;
use crate::jobs::Queue;
use crate::library::Library;
use crate::renders::RenderCache;

/// What every handler can reach.
///
/// Cheap to clone — a pool is a handle — which is what axum asks of state.
#[derive(Clone)]
pub struct AppState {
    /// The database, as [`db::member_pool`](crate::db::member_pool) makes
    /// it: every connection unable to read a table until a handler opens a
    /// scoped or privileged transaction.
    pub pool: PgPool,
    /// The live stream every user's browser listens on.
    pub events: Events,
    /// The job queue, for announcing a job once it is enqueued.
    pub jobs: Queue,
    /// Every user's files.
    pub library: Library,
    /// Finished renders, and which of them are being downloaded.
    pub renders: RenderCache,
}

impl AppState {
    /// State answering from `pool` with users' files in `files`, with a new
    /// event bus and a queue telling it.
    pub fn new(pool: PgPool, files: Files) -> Self {
        let events = Events::new();
        let jobs = Queue::new(events.clone());
        Self {
            library: Library::new(pool.clone(), files.storage, files.tools, jobs.clone()),
            renders: files.renders,
            pool,
            jobs,
            events,
        }
    }
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
        .route("/tokens/{id}", delete(tokens::revoke))
        .route("/jobs", get(jobs::list))
        .route("/jobs/{id}", get(jobs::get))
        .route("/events", get(events::stream))
        .route("/projects", get(projects::list).post(projects::create))
        .route(
            "/projects/{id}",
            get(projects::open)
                .put(projects::save)
                .patch(projects::rename)
                .delete(projects::delete),
        )
        .merge(credit_routes())
        .merge(library_routes())
        .merge(render_routes());
    Router::new().nest("/api", api).with_state(state)
}

/// The library's routes (#535): its files, and the tus uploads that fill it.
fn library_routes() -> Router<AppState> {
    Router::new()
        .route("/library", get(library::list))
        .route(
            "/library/{id}",
            get(library::details)
                .patch(library::update)
                .delete(library::delete),
        )
        .route("/library/{id}/file", get(library::file))
        .route("/library/{id}/thumbnail", get(library::thumbnail))
        .route(
            "/uploads",
            post(uploads::announce).options(uploads::options),
        )
        .route(
            "/uploads/{id}",
            head(uploads::progress)
                .patch(uploads::append)
                .delete(uploads::cancel),
        )
}

/// The credit routes (#537), kept apart so that the routes each feature adds
/// sit in a block of their own rather than one long chain every branch edits.
fn credit_routes() -> Router<AppState> {
    Router::new()
        .route("/credits", get(credits::balance))
        .route("/credits/history", get(credits::history))
}

/// The render routes (#541): ask for a render, list a project's, download one.
fn render_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{id}/renders",
            get(renders::list).post(renders::request),
        )
        .route("/renders/{id}/file", get(renders::file))
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
