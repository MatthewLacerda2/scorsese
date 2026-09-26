//! The accounts themselves: what the operator does to them, and logging in.
//!
//! Everything here except [`change_password`] is cross-user by nature — the
//! operator names an account by email, and a login does not yet know whose it
//! is — so it runs [`privileged`](crate::db::privileged). That is the short,
//! reviewable list the isolation doc promises.

use serde::Serialize;
use sqlx::postgres::PgPool;

use super::{AccountError, normalize_email, password};
use crate::db::{self, UserId};
use crate::storage::Storage;

/// An account as the operator's listing shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    /// Its id, which is also the name of its directory under the storage root.
    pub id: i64,
    /// The normalised email it logs in with.
    pub email: String,
    /// When it was created, in seconds since the Unix epoch.
    pub created_at: i64,
}

/// Create an account. The email is normalised; the password is hashed.
pub async fn create(pool: &PgPool, email: &str, password: &str) -> Result<UserId, AccountError> {
    let email = normalize_email(email)?;
    let hash = password::hash_blocking(password.to_owned()).await?;
    let mut tx = db::privileged(pool).await?;
    let id: Option<i64> = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2)
         ON CONFLICT (email) DO NOTHING RETURNING id",
    )
    .bind(&email)
    .bind(hash)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    id.map(UserId::from_row).ok_or(AccountError::Taken(email))
}

/// Replace an account's password, and end every session it has.
///
/// The operator's reset: whoever asked has lost the password or thinks
/// somebody else has it, and in the second case a browser still logged in
/// is exactly what must stop working. API tokens are left alone — each is
/// revoked on its own, by name.
pub async fn set_password(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<UserId, AccountError> {
    let email = normalize_email(email)?;
    let hash = password::hash_blocking(password.to_owned()).await?;
    let mut tx = db::privileged(pool).await?;
    let id: Option<i64> =
        sqlx::query_scalar("UPDATE users SET password_hash = $2 WHERE email = $1 RETURNING id")
            .bind(&email)
            .bind(hash)
            .fetch_optional(&mut *tx)
            .await?;
    let id = id.ok_or(AccountError::NoSuchAccount(email))?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(UserId::from_row(id))
}

/// Delete an account, everything in the database that is theirs, and their
/// files.
///
/// The rows go by `ON DELETE CASCADE` — every per-user table has one, and
/// `tests/isolation.rs` refuses a table that does not — so this names only
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

/// Every account, oldest first.
pub async fn list(pool: &PgPool) -> Result<Vec<Account>, AccountError> {
    let mut tx = db::privileged(pool).await?;
    let rows: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT id, email, extract(epoch FROM created_at)::bigint FROM users ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|(id, email, created_at)| Account {
            id,
            email,
            created_at,
        })
        .collect())
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
    let (id, email, created_at) =
        sqlx::query_as("SELECT id, email, extract(epoch FROM created_at)::bigint FROM users")
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(Account {
        id,
        email,
        created_at,
    })
}

/// Whose account `email` and `password` open, if any.
///
/// `None` for an unknown email and a wrong password alike, in the same time
/// (see [`password::verify_blocking`]): a login form that says which of the two
/// was wrong is a way to find out who has an account.
pub async fn authenticate(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<Option<UserId>, AccountError> {
    let row: Option<(i64, String)> = match normalize_email(email) {
        Ok(email) => {
            let mut tx = db::privileged(pool).await?;
            let row = sqlx::query_as("SELECT id, password_hash FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(&mut *tx)
                .await?;
            tx.commit().await?;
            row
        }
        Err(_) => None,
    };
    let (id, stored) = row.unzip();
    let matches = password::verify_blocking(password.to_owned(), stored).await;
    Ok(id.filter(|_| matches).map(UserId::from_row))
}

/// A user changing their own password, given the current one.
pub async fn change_password(
    pool: &PgPool,
    user: UserId,
    current: &str,
    new: &str,
) -> Result<(), AccountError> {
    let new = password::acceptable(new)?;
    let mut tx = db::scoped(pool, user).await?;
    // No WHERE, here or in the UPDATE: the scope shows one row, theirs.
    let stored: String = sqlx::query_scalar("SELECT password_hash FROM users")
        .fetch_one(&mut *tx)
        .await?;
    if !password::verify_blocking(current.to_owned(), Some(stored)).await {
        return Err(AccountError::WrongPassword);
    }
    let hash = password::hash_blocking(new.to_owned()).await?;
    sqlx::query("UPDATE users SET password_hash = $1")
        .bind(hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
