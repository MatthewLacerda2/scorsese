//! The login's brake (#557): too many attempts for one email, or from one
//! client address, and the next ones are refused for a while.
//!
//! **Two counters per attempt, and either one refuses.** The email's catches
//! a patient attacker spread over many addresses; the address's catches one
//! source spraying guesses across many emails, which a per-email count never
//! sees. Both follow the same rules ([`counter`]): ten attempts in fifteen
//! minutes, then a fifteen-minute lock-out that doubles each time it recurs,
//! up to a day, and is forgiven after a quiet day.
//!
//! **It says nothing about who has an account.** A counter is kept for
//! whatever string was typed, and nothing here looks at `users`, so an email
//! nobody has is counted, locked and answered exactly like a real one. A
//! refused attempt never reaches argon2 whichever it is, so the timing rule
//! the login keeps — an unknown email and a wrong password answered in the
//! same time — holds on both sides of the lock.
//!
//! **In Postgres, not in the server's memory.** One process on one machine
//! could keep this in a map, as web MCP's per-minute limit does — but the
//! operator's `scorsese-server user locks` and `user unlock` run as a second
//! process, which could neither see nor clear a map in the first; a
//! restart, which a crash or a deploy is, would forgive every lock-out and
//! every strike; and the cost is two indexed rows per login attempt, which
//! at this scale is nothing. Rows are forgotten after a quiet day, so the
//! table holds what happened lately and not a history of who tried what.
//!
//! Everything here is cross-user by nature — an attempt is nobody's until it
//! succeeds — so it runs [`privileged`](crate::db::privileged).

pub mod counter;
pub mod key;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::postgres::PgPool;

use crate::db;
pub use counter::Counter;
pub use key::Key;

/// A counter as the operator's listing shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    /// Whose.
    pub key: Key,
    /// Attempts counted in the current window.
    pub attempts: u32,
    /// Seconds until the lock-out ends; 0 when there is none.
    pub locked_for: i64,
    /// Lock-outs in a row.
    pub strikes: u32,
}

/// Count an attempt against every one of `keys`, or refuse it: `Err` with how
/// long until the longest lock-out among them ends.
///
/// All or nothing: an attempt one counter refuses is not counted against the
/// others, so a locked address cannot run a stranger's email counter up. A
/// refusal that *starts* a lock-out is kept, as is when each was last tried.
pub async fn admit(pool: &PgPool, keys: &[Key]) -> Result<Result<(), Duration>, sqlx::Error> {
    let now = now();
    let mut keys = keys.to_vec();
    // One order for every transaction, so two logins never wait on each
    // other's rows the other way round.
    keys.sort();
    keys.dedup();
    let mut tx = db::privileged(pool).await?;
    sqlx::query("DELETE FROM login_throttle WHERE touched < $1 AND locked_until < $1")
        .bind(now - counter::FORGIVEN_AFTER)
        .execute(&mut *tx)
        .await?;
    let mut counters = Vec::with_capacity(keys.len());
    for key in &keys {
        counters.push(load(&mut tx, key, now).await?);
    }
    let mut after = counters.clone();
    let verdicts: Vec<_> = after.iter_mut().map(|c| c.attempt(now)).collect();
    let wait = verdicts.iter().filter_map(|v| v.err()).max();
    for (index, key) in keys.iter().enumerate() {
        // Admitted everywhere: keep every count. Refused somewhere: keep only
        // what the refusing counters did, and leave the others as they were.
        let keep = wait.is_none() || verdicts[index].is_err();
        store(
            &mut tx,
            key,
            if keep {
                &after[index]
            } else {
                &counters[index]
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(match wait {
        Some(seconds) => Err(Duration::from_secs(seconds.max(1).unsigned_abs())),
        None => Ok(()),
    })
}

/// A login that `email` and `address` admitted succeeded: the email's counter
/// is cleared — the account's owner has just proved who they are — and the
/// address gets its attempt back but keeps its strikes, so logging in to an
/// account of one's own between guesses at others' buys nothing.
pub async fn succeeded(
    pool: &PgPool,
    email: &Key,
    address: Option<&Key>,
) -> Result<(), sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    delete(&mut tx, email).await?;
    if let Some(address) = address {
        sqlx::query(
            "UPDATE login_throttle SET attempts = GREATEST(attempts - 1, 0)
             WHERE kind = $1 AND key = $2",
        )
        .bind(address.kind())
        .bind(address.text())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// Every counter locked now or counting attempts in its current window, for
/// the operator: addresses first, then emails.
pub async fn locks(pool: &PgPool) -> Result<Vec<Lock>, sqlx::Error> {
    let now = now();
    let mut tx = db::privileged(pool).await?;
    let rows: Vec<(String, String, i32, i64, i32)> = sqlx::query_as(
        "SELECT kind, key, attempts, locked_until, strikes FROM login_throttle
         WHERE locked_until > $1 OR (attempts > 0 AND window_start > $1 - $2)
         ORDER BY kind, key",
    )
    .bind(now)
    .bind(counter::WINDOW)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|(kind, text, attempts, locked_until, strikes)| Lock {
            key: if kind == "email" {
                Key::Email(text)
            } else {
                Key::Address(text)
            },
            attempts: attempts.unsigned_abs(),
            locked_for: (locked_until - now).max(0),
            strikes: strikes.unsigned_abs(),
        })
        .collect())
}

/// Forget everything about `key` — its count, its lock-out and its strikes.
/// Whether there was anything to forget.
pub async fn clear(pool: &PgPool, key: &Key) -> Result<bool, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let cleared = delete(&mut tx, key).await?;
    tx.commit().await?;
    Ok(cleared)
}

/// `key`'s counter, locked for the rest of the transaction — a fresh one if it
/// has none yet.
async fn load(tx: &mut db::Tx, key: &Key, now: i64) -> Result<Counter, sqlx::Error> {
    // Insert first, then lock: two first attempts arriving together would
    // otherwise both find no row and both try to create it.
    sqlx::query(
        "INSERT INTO login_throttle (kind, key, window_start, touched) VALUES ($1, $2, $3, $3)
         ON CONFLICT DO NOTHING",
    )
    .bind(key.kind())
    .bind(key.text())
    .bind(now)
    .execute(&mut **tx)
    .await?;
    let (attempts, window_start, locked_until, strikes, touched): (i32, i64, i64, i32, i64) =
        sqlx::query_as(
            "SELECT attempts, window_start, locked_until, strikes, touched FROM login_throttle
             WHERE kind = $1 AND key = $2 FOR UPDATE",
        )
        .bind(key.kind())
        .bind(key.text())
        .fetch_one(&mut **tx)
        .await?;
    Ok(Counter {
        attempts: attempts.unsigned_abs(),
        window_start,
        locked_until,
        strikes: strikes.unsigned_abs(),
        touched,
    })
}

/// Write `counter` back as `key`'s.
async fn store(tx: &mut db::Tx, key: &Key, counter: &Counter) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE login_throttle
         SET attempts = $3, window_start = $4, locked_until = $5, strikes = $6, touched = $7
         WHERE kind = $1 AND key = $2",
    )
    .bind(key.kind())
    .bind(key.text())
    .bind(i32::try_from(counter.attempts).unwrap_or(i32::MAX))
    .bind(counter.window_start)
    .bind(counter.locked_until)
    .bind(i32::try_from(counter.strikes).unwrap_or(i32::MAX))
    .bind(counter.touched)
    .execute(&mut **tx)
    .await
    .map(drop)
}

/// Delete `key`'s row; whether there was one.
async fn delete(tx: &mut db::Tx, key: &Key) -> Result<bool, sqlx::Error> {
    let deleted = sqlx::query("DELETE FROM login_throttle WHERE kind = $1 AND key = $2")
        .bind(key.kind())
        .bind(key.text())
        .execute(&mut **tx)
        .await?;
    Ok(deleted.rows_affected() > 0)
}

/// Seconds since the Unix epoch, by this machine's clock.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}
