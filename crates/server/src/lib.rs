//! # scorsese-server — the web API
//!
//! Responsibility: serving scorsese over HTTP to the web app (#527), and
//! owning the Postgres database everything around a user's edit lives in —
//! accounts, their library, their projects, jobs, credits. One more thin
//! client of the library crates, a sibling of `scorsese-cli` and
//! `scorsese-mcp`.
//!
//! Boundary: **no editing logic of its own.** An endpoint that needs code the
//! CLI does not share is the sign that code belongs in `scorsese-core`,
//! `scorsese-render` or `scorsese-providers`, and it moves there first. What
//! *is* this crate's own is what only a hosted service has: HTTP, the
//! database, per-user isolation, and money. It never touches a display, never
//! runs ffmpeg itself (renders go through `scorsese-render`, whose command
//! builder is the one place ffmpeg is invoked), and never calls a provider
//! except through `scorsese-providers`. Nothing in the workspace depends on
//! this crate.
//!
//! ## axum and sqlx
//!
//! **axum** because it is the tokio project's own framework: the runtime sqlx
//! needs anyway, `tower` middleware for the auth and rate limiting later
//! issues add, and no macros between a handler and what it is — a handler is
//! an `async fn`. **sqlx** because it is SQL rather than a query DSL — a table
//! here is read in the language it was written in — with migrations built in
//! and embedded in the binary ([`db`]), and a test harness (`#[sqlx::test]`)
//! that gives every test a database of its own. Its compile-checked
//! `query!` macros are deliberately **not used** (decided with the first real
//! queries, #533): they need a live database or a checked-in `.sqlx/` cache
//! at build time — one more thing for every build and every agent's worktree
//! to keep in step — while every query here already runs against a real
//! Postgres in the test gate, which catches a typo'd column just as loudly
//! and also catches what a type check cannot, like a policy refusing a row.
//!
//! Both are default-features-off with only what is used switched on. No TLS
//! to Postgres: it runs beside the server on one host, reached over the
//! compose network, and a TLS stack is a dependency tree `cargo deny` would
//! have to keep clearing for a connection that never leaves the machine.
//!
//! ## The tool registry is `scorsese-mcp`'s, and this crate depends on it
//!
//! Decided here because every later issue inherits it (#530). The built-in
//! assistant (#540) and web MCP (#539) both need the tools `scorsese-mcp`
//! defines, and they will reach them by **depending on `scorsese-mcp`** — not
//! by moving the registry into a new crate beneath both. The dependency is
//! added by the first issue that calls a tool, not before.
//!
//! - The registry is not the whole of what is shared. Web MCP is the same
//!   JSON-RPC over a different transport, so it wants `scorsese-mcp`'s
//!   dispatch as well as its tools; a registry crate beneath both would leave
//!   that behind, and a transport-agnostic "answer one message" entry point in
//!   `scorsese-mcp` is the smaller change.
//! - The gates travel with the tools where they already are: every tool and
//!   argument described, `docs/mcp.md`'s table generated from the registry,
//!   each tool's declared cost. Moving the registry moves all of it and buys
//!   a crate whose only difference from `scorsese-mcp` is three small
//!   protocol files.
//! - It costs this crate next to nothing: `scorsese-mcp` is hand-rolled, and
//!   beyond `serde` it depends only on the library crates this one is a
//!   client of anyway.
//! - It does not bend the boundary above. The editing logic a tool performs
//!   already lives in `core`, `render` and `providers`; what `scorsese-mcp`
//!   adds on top is protocol — each tool's self-description and the message
//!   handling around it — which is exactly what a second transport for the
//!   same tools should reuse rather than re-derive.
//!
//! The constraint every later issue inherits: **`scorsese-mcp` never learns
//! about users or Postgres.** It stays stateless and database-free, and
//! whatever turns a user's project id into something a tool can act on lives
//! here and is handed to the tool. The day that cannot hold — a tool that
//! needs the database to do its job — is the day to move the registry down a
//! layer after all, and this paragraph is where to say so.
//!
//! ## Database tests run, or they fail — they never skip
//!
//! A test that needs Postgres is a `#[sqlx::test]`, which creates a fresh
//! database per test on the server `DATABASE_URL` names and drops it after.
//! Without `DATABASE_URL` such a test **fails**; it does not skip. `make test`
//! provides one: `tools/with-postgres` starts a throwaway Postgres container
//! for the length of the run, or uses `SCORSESE_TEST_DATABASE_URL` when that
//! is set; CI runs a Postgres service container. So `make gates` stays the
//! complete answer to what CI blocks on, and Postgres joins ffmpeg as a thing
//! the test gate needs rather than a thing it quietly works around.
//!
//! ## What this publishes
//!
//! [`run`], which is what the binary calls to serve, and the parts it is
//! assembled from — [`Config`], [`db`], [`http`] and [`start`] — published
//! because the tests drive each one from outside the crate. [`accounts`] is
//! who may use the server, [`operator`] the commands that manage them, and
//! [`storage`] where each user's files live.

pub mod accounts;
pub mod config;
pub mod db;
pub mod http;
pub mod operator;
pub mod storage;

use std::future::Future;
use std::path::PathBuf;

use sqlx::postgres::PgPool;
use tokio::net::TcpListener;

pub use accounts::AccountError;
pub use config::{Config, ConfigError};

use crate::http::AppState;

/// Why the server stopped, or never started.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// The environment does not describe a server that can start.
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// The database could not be reached.
    #[error("could not connect to the database: {0}")]
    Connect(#[source] sqlx::Error),

    /// The schema could not be brought up to date.
    #[error("could not migrate the database: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// The storage root could not be created.
    #[error("could not create the storage directory {}: {source}", path.display())]
    Storage {
        /// The directory that could not be created.
        path: PathBuf,
        /// What the filesystem said.
        #[source]
        source: std::io::Error,
    },

    /// An account command could not be carried out.
    #[error(transparent)]
    Account(#[from] AccountError),

    /// An irreversible command was not confirmed; the message says how.
    #[error("{0}")]
    Unconfirmed(String),

    /// The listen address could not be bound, or serving failed.
    #[error("could not serve HTTP: {0}")]
    Serve(#[from] std::io::Error),
}

/// Start the server this configuration describes and run it until `shutdown`.
///
/// Connect, create the storage root, bind, then [`start`]. Every step that
/// can fail does so before the first request is accepted.
pub async fn run(
    config: Config,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    let pool = db::connect(&config).await.map_err(ServerError::Connect)?;
    std::fs::create_dir_all(&config.storage).map_err(|source| ServerError::Storage {
        path: config.storage.clone(),
        source,
    })?;
    let listener = TcpListener::bind(config.bind).await?;
    start(pool, listener, shutdown).await
}

/// Migrate, then serve on `listener` until `shutdown`.
///
/// The half of [`run`] that is handed its resources rather than making them,
/// so a test can give it a database of its own and a port the OS picked.
/// Migrations run before the first connection is accepted, so no request ever
/// meets a schema older than the code answering it. Requests are then
/// answered from [`db::member_pool`], whose connections can read nothing
/// outside a scoped transaction — per-user isolation, `db::scope`.
pub async fn start(
    pool: PgPool,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    db::migrate(&pool).await?;
    let pool = db::member_pool(&pool).await.map_err(ServerError::Connect)?;
    http::serve(listener, http::router(AppState { pool }), shutdown).await?;
    Ok(())
}

/// Connect to the configured database and bring its schema up to date.
///
/// What an operator command starts with: the same connection and the same
/// migrations the server runs, so a command run against a fresh database
/// works rather than finding no `users` table.
pub async fn open_database(config: &Config) -> Result<PgPool, ServerError> {
    let pool = db::connect(config).await.map_err(ServerError::Connect)?;
    db::migrate(&pool).await?;
    Ok(pool)
}
