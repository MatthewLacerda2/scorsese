//! The accounts themselves: what the operator does to them, and signing in.
//!
//! Everything here except [`get`] is cross-user by nature — the operator
//! names an account by email, and a sign-in does not yet know whose it is —
//! so it runs [`privileged`](crate::db::privileged). That is the short,
//! reviewable list the isolation doc promises.

use serde::Serialize;
use sqlx::postgres::PgPool;

use scorsese_providers::sign_in::Identity;

use super::{AccountError, normalize_email};
use crate::db::{self, UserId};
use crate::storage::Storage;

/// An account as the operator's listing shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct Account {
    /// Its id, which is also the name of its directory under the storage root.
    pub id: i64,
    /// The normalised email it logs in with.
    pub email: String,
    /// The person's name, from their Google account (#1266); `None` until
    /// their first sign-in, or when Google gives none — and then left out of
    /// the JSON rather than sent as `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// When it was created, in seconds since the Unix epoch.
    pub created_at: i64,
}

/// Create an account for `email`, to be linked by its owner's first Google
/// sign-in under that address — the operator's way to set an account up (and
/// top it up) before its owner arrives.
pub async fn create(pool: &PgPool, email: &str) -> Result<UserId, AccountError> {
    let email = normalize_email(email)?;
    let mut tx = db::privileged(pool).await?;
    let id: Option<i64> = sqlx::query_scalar(
        "INSERT INTO users (email) VALUES ($1) ON CONFLICT (email) DO NOTHING RETURNING id",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    id.map(UserId::from_row).ok_or(AccountError::Taken(email))
}

/// Delete an account, everything in the database that is theirs, and their
/// files.
///
/// The rows go by `ON DELETE CASCADE` — every per-user table has one, and
/// `tests/suite/isolation.rs` refuses a table that does not — so this names only
/// `users`, and a table added next year is covered without touching it.
/// Rows first, then files: an account whose files outlived it is an operator
/// cleaning up a named directory, while files deleted under a live account
/// would be somebody's work gone with no record of why.
pub async fn delete(pool: &PgPool, storage: &Storage, email: &str) -> Result<UserId, AccountError> {
    let email = normalize_email(email)?;
    let mut tx = db::privileged(pool).await?;
    let id: Option<i64> = sqlx::query_scalar("DELETE FROM users WHERE email = $1 RETURNING id")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?;
    let user = UserId::from_row(id.ok_or(AccountError::NoSuchAccount(email))?);
    tx.commit().await?;
    storage
        .remove_user(user)
        .map_err(|(path, source)| AccountError::Files { path, source })?;
    Ok(user)
}

/// An account as the operator's `user list` shows it: with the Google
/// account it signs in as, which only the operator needs to see.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Listed {
    /// The account.
    #[sqlx(flatten)]
    pub account: Account,
    /// Google's id for the account it is linked to; `None` until its first
    /// sign-in.
    pub google_sub: Option<String>,
}

/// Every account, oldest first.
pub async fn list(pool: &PgPool) -> Result<Vec<Listed>, AccountError> {
    let mut tx = db::privileged(pool).await?;
    let listed = sqlx::query_as(
        "SELECT id, email, name, extract(epoch FROM created_at)::bigint AS created_at, google_sub
         FROM users ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(listed)
}

/// The account `email` names — for the operator, who names people by email.
pub async fn find(pool: &PgPool, email: &str) -> Result<UserId, AccountError> {
    let email = normalize_email(email)?;
    let mut tx = db::privileged(pool).await?;
    let id: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?;
    tx.commit().await?;
    id.map(UserId::from_row)
        .ok_or(AccountError::NoSuchAccount(email))
}

/// `user`'s own account — what `GET /api/me` answers.
pub async fn get(pool: &PgPool, user: UserId) -> Result<Account, AccountError> {
    let mut tx = db::scoped(pool, user).await?;
    // No WHERE: the scope shows exactly one row of `users`, their own.
    let (id, email, name, created_at) =
        sqlx::query_as("SELECT id, email, name, extract(epoch FROM created_at)::bigint FROM users")
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(Account {
        id,
        email,
        name,
        created_at,
    })
}

/// Which account a Google sign-in opened, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedIn {
    /// The account already linked to this Google account.
    Returning(UserId),
    /// An account under the same verified email, linked from now on.
    Linked(UserId),
    /// A new account, made by this sign-in, with no credits.
    Created(UserId),
}

impl SignedIn {
    /// The account, however it was reached.
    pub fn user(self) -> UserId {
        match self {
            Self::Returning(user) | Self::Linked(user) | Self::Created(user) => user,
        }
    }
}

/// The account Google's `identity` opens: the one linked to it; else the one
/// under its verified email, which it is linked to from now on; else a new
/// one (#952, *Decisions* 1 and 2).
///
/// After the link, `sub` is the identity and the email is never looked at
/// again — so somebody who later changes the address on their Google account
/// keeps their scorsese account, and somebody handed that old address does
/// not get it. An account already linked to a *different* Google account is
/// refused rather than taken over: that is the operator's to sort out.
///
/// Every sign-in also writes the name Google gave (#1266), so a name changed
/// at Google follows. A sign-in that carries none keeps the one stored.
pub async fn sign_in(pool: &PgPool, identity: &Identity) -> Result<SignedIn, AccountError> {
    let mut tx = db::privileged(pool).await?;
    let linked: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE google_sub = $1")
        .bind(&identity.sub)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(id) = linked {
        rename(&mut tx, id, identity).await?;
        tx.commit().await?;
        return Ok(SignedIn::Returning(UserId::from_row(id)));
    }
    let email = normalize_email(&identity.email)?;
    let existing: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT id, google_sub FROM users WHERE email = $1 FOR UPDATE")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?;
    let signed = match existing {
        Some((_, Some(_))) => return Err(AccountError::LinkedElsewhere(email)),
        Some((id, None)) => {
            sqlx::query("UPDATE users SET google_sub = $2 WHERE id = $1")
                .bind(id)
                .bind(&identity.sub)
                .execute(&mut *tx)
                .await?;
            rename(&mut tx, id, identity).await?;
            SignedIn::Linked(UserId::from_row(id))
        }
        None => {
            let id: Option<i64> = sqlx::query_scalar(
                "INSERT INTO users (email, google_sub, name) VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING RETURNING id",
            )
            .bind(&email)
            .bind(&identity.sub)
            .bind(&identity.name)
            .fetch_optional(&mut *tx)
            .await?;
            // Nothing returned: the same person's other tab won the race.
            SignedIn::Created(UserId::from_row(id.ok_or(AccountError::Taken(email))?))
        }
    };
    tx.commit().await?;
    Ok(signed)
}

/// Write the name `identity` carries onto account `id`, keeping the stored one
/// when it carries none.
async fn rename(tx: &mut db::Tx, id: i64, identity: &Identity) -> Result<(), AccountError> {
    sqlx::query("UPDATE users SET name = COALESCE($2, name) WHERE id = $1")
        .bind(id)
        .bind(&identity.name)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Link the account `email` names to the Google account `sub` — the
/// operator's answer for somebody whose Google email is not the one their
/// account was made under. Replaces any link it had.
pub async fn link(pool: &PgPool, email: &str, sub: &str) -> Result<UserId, AccountError> {
    let email = normalize_email(email)?;
    let sub = sub.trim();
    let mut tx = db::privileged(pool).await?;
    let holder: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE google_sub = $1")
        .bind(sub)
        .fetch_optional(&mut *tx)
        .await?;
    let id: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?;
    let id = id.ok_or(AccountError::NoSuchAccount(email))?;
    match holder {
        Some(holder) if holder != id => return Err(AccountError::SubTaken(holder)),
        _ => {}
    }
    sqlx::query("UPDATE users SET google_sub = $2 WHERE id = $1")
        .bind(id)
        .bind(sub)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(UserId::from_row(id))
}
