//! The access record (#1267): which account was used from which address, and
//! when — what Marco Civil da Internet (Lei 12.965/2014) art. 15 asks an
//! application run by a company for profit to keep, for at least six months.
//!
//! Two moments are recorded, never every request:
//!
//! - **each sign-in** ([`Access::SignIn`]), when a session is opened;
//! - **an account's first request of each day**, in UTC ([`Access::Day`]), by
//!   session or by API token alike — so a month-long session, or a token an
//!   MCP client uses daily, still leaves a row per day it was used.
//!
//! The table is `access_log` (migration `0033`). It is **not per user**: the
//! law says the record outlives the account, so a deletion leaves it, and the
//! table carries no cascade. Writing is privileged, and nothing in the server
//! reads it — the operator does, with `psql` (`docs/web.md`, *The access
//! record*). Rows are **never pruned** (the maintainer, 2026-10-14): six
//! months is the minimum, not a ceiling.
//!
//! The address is the visitor's as the deploy's nginx names it
//! (`X-Scorsese-Client`, `http::visitor`), which is Cloudflare's
//! `CF-Connecting-IP` for a request through the tunnel.
//!
//! **A failed write never fails the request.** It is logged, and a day not
//! recorded is tried again on the account's next request; turning a user away
//! because a log row could not be written would be the wrong way round.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::postgres::PgPool;

use crate::db::{self, UserId};

/// Which moment a row records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// A session opened by signing in.
    SignIn,
    /// The account's first request of the day.
    Day,
}

impl Access {
    /// The `kind` column's value.
    fn kind(self) -> &'static str {
        match self {
            Self::SignIn => "sign_in",
            Self::Day => "day",
        }
    }
}

/// Record that `user` accessed the service from `address`, now.
///
/// A second [`Access::Day`] row for the same account and day is ignored, so
/// this is safe to call more than once a day — after a restart, or from two
/// requests racing.
pub async fn record(
    pool: &PgPool,
    user: UserId,
    address: Option<IpAddr>,
    access: Access,
) -> Result<(), sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    sqlx::query(
        "INSERT INTO access_log (user_id, kind, address) VALUES ($1, $2, $3::inet)
         ON CONFLICT (user_id, day) WHERE kind = 'day' DO NOTHING",
    )
    .bind(user.get())
    .bind(access.kind())
    .bind(address.map(|address| address.to_string()))
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

/// [`record`], logging a failure instead of returning it.
pub async fn record_or_log(
    pool: &PgPool,
    user: UserId,
    address: Option<IpAddr>,
    access: Access,
) -> bool {
    match record(pool, user, address, access).await {
        Ok(()) => true,
        Err(error) => {
            eprintln!(
                "scorsese-server: could not record account {}'s access: {error}",
                user.get()
            );
            false
        }
    }
}

/// Which day each account was last recorded on, so the database is asked
/// once per account per day rather than on every request.
///
/// In memory, and lost on a restart: the first request after one writes again,
/// and the table's unique index makes that a no-op.
#[derive(Debug, Clone, Default)]
pub struct Daily(Arc<Mutex<HashMap<UserId, u64>>>);

impl Daily {
    /// Record `user`'s request from `address` if it is their first today.
    pub async fn note(&self, pool: &PgPool, user: UserId, address: Option<IpAddr>) {
        let today = today();
        if self.recorded(user) == Some(today) {
            return;
        }
        if record_or_log(pool, user, address, Access::Day).await {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(user, today);
        }
    }

    fn recorded(&self, user: UserId) -> Option<u64> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&user)
            .copied()
    }
}

/// Days since the epoch, in UTC — the same day the table's `day` column
/// counts.
fn today() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() / 86_400)
}
