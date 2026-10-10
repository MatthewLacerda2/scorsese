//! The HTTP surface: the routes, and serving them until told to stop.
//!
//! Every route is declared once, in [`routes::table`], and `docs/web.md`'s
//! route tables are held to it — that page is where each one is described.
//!
//! "A member" is a request carrying a session cookie or an API token — see
//! [`auth`].

pub mod account;
pub mod auth;
pub mod chat;
pub mod client;
pub mod credits;
mod disposition;
pub mod editor;
pub mod error;
pub mod events;
pub mod jobs;
pub mod library;
pub mod mcp;
pub mod projects;
mod ranges;
pub mod renders;
pub mod routes;
pub mod styles;
pub mod templates;
pub mod tokens;
pub mod uploads;

use std::future::Future;
use std::net::SocketAddr;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use sqlx::postgres::PgPool;
use tokio::net::TcpListener;

use crate::Files;
use crate::assistant::Assistant;
use crate::events::Events;
use crate::jobs::Queue;
use crate::library::Library;
use crate::renders::RenderCache;
use crate::tools::Toolbox;

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
    /// scorsese's tools, for any user: what web MCP serves and the
    /// built-in assistant (#540) calls in-process.
    pub tools: Toolbox,
    /// How many tool calls each user has made lately, over web MCP.
    pub limits: mcp::Limits,
    /// The web MCP calls each user has running, for a cancel to find.
    pub in_flight: mcp::InFlight,
    /// The built-in assistant (#540): how it reaches Claude, and what a turn
    /// may cost.
    pub assistant: Assistant,
    /// Which address a request is counted as coming from, for the login's
    /// brake: the peer, or the header the deploy's nginx writes.
    pub clients: client::Clients,
}

impl AppState {
    /// State answering from `pool` with users' files in `files`, with a new
    /// event bus and a queue telling it.
    pub fn new(pool: PgPool, files: Files) -> Self {
        let events = Events::new();
        let jobs = Queue::new(events.clone());
        let library = Library::new(
            pool.clone(),
            files.storage,
            files.tools.clone(),
            jobs.clone(),
        );
        let tools = Toolbox::new(
            pool.clone(),
            library.clone(),
            files.tools,
            jobs.clone(),
            files.renders.clone(),
        );
        Self {
            library,
            renders: files.renders,
            tools,
            limits: mcp::Limits::default(),
            in_flight: mcp::InFlight::default(),
            assistant: Assistant::default(),
            clients: client::Clients::default(),
            pool,
            jobs,
            events,
        }
    }
}

impl AppState {
    /// The same state with `assistant` as its assistant.
    pub fn with_assistant(mut self, assistant: Assistant) -> Self {
        self.assistant = assistant;
        self
    }

    /// The same state with its tools searching and importing stock media from
    /// `library` alone — a test's, which needs no key and no network.
    pub fn stocked_from(mut self, library: scorsese_mcp::Stock) -> Self {
        self.tools = self.tools.stocked_from(library);
        self
    }

    /// The same state, believing `clients` for where a request came from.
    pub fn with_clients(mut self, clients: client::Clients) -> Self {
        self.clients = clients;
        self
    }
}

/// Every route the server answers, all of them under `/api`.
///
/// One prefix for the whole API, health check included, so the web app's
/// dev proxy (`web/`, which forwards `/api` unchanged) and whatever fronts the
/// server in production route it with a single rule — and a path that is not
/// `/api/...` is never this server's, so it can go to the front-end.
pub fn router(state: AppState) -> Router {
    let api = routes::table()
        .into_iter()
        .fold(Router::new(), |api, route| {
            api.route(route.path, route.handler)
        });
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
///
/// Each request carries the address it was connected from, which is what
/// [`client::ClientAddress`] reads when nothing stands in front of the server.
pub async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let service = router.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, service)
        .with_graceful_shutdown(shutdown)
        .await
}
