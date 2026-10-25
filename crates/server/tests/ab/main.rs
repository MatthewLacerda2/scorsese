//! The landing's A/B tests (#1553, `ab`): what a visitor's browser posts is
//! kept within its caps, tied to an account at sign-in (`tests/google/ab.rs`)
//! and let go of when the account is deleted, and read back by the report.

#[path = "../common/mod.rs"]
mod common;

mod caps;
mod events;
mod report;

use sqlx::postgres::PgPool;

/// A visitor code, as the web makes them.
const ANA: &str = "0b7f6a3e-9c1d-4e2f-8a5b-6c7d8e9f0a1b";

/// The route.
const PATH: &str = "/api/ab/events";

/// `(kind, value)` of every event, oldest first.
async fn events(pool: &PgPool) -> Vec<(String, i32)> {
    sqlx::query_as("SELECT kind, value FROM ab_events ORDER BY id")
        .fetch_all(pool)
        .await
        .expect("the events read")
}
