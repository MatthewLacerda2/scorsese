//! API tokens: how a client that is not a browser acts as a user.
//!
//! **Per-user bearer tokens for v1, OAuth later.** The MCP specification's
//! remote authorization is OAuth 2.1, and the clients people will actually
//! point at web MCP first — Claude Code, scripts, other agents' MCP
//! configuration — all accept a static `Authorization: Bearer` header. A
//! token is a row and a hash; an OAuth authorization server is dynamic client
//! registration, consent screens, refresh tokens and PKCE, which is a large
//! surface to get right for no client that needs it yet. The day a client
//! that *only* speaks OAuth matters — claude.ai's custom connectors are the
//! likely one — it is its own issue, and it issues these same tokens at the
//! end of its flow, so nothing downstream of "who is this" changes.
//!
//! A token acts as its user in full; there are no scopes, because there is
//! nothing yet worth scoping to. It is shown once, when issued, and is
//! revocable on its own by id, so a leaked one costs one revocation rather
//! than a password reset.

use serde::Serialize;
use sqlx::postgres::PgPool;

use super::{AccountError, secret};
use crate::db::{self, UserId};

/// A token as its owner's list shows it — never the token itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenInfo {
    /// What to revoke it by.
    pub id: i64,
    /// What its owner called it, e.g. "laptop claude code".
    pub name: String,
    /// When it was issued, in seconds since the Unix epoch.
    pub created_at: i64,
    /// When it last authenticated a request, if ever.
    pub last_used_at: Option<i64>,
}

/// A token just issued: the only time its value exists outside its owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issued {
    /// What to revoke it by.
    pub id: i64,
    /// The bearer value. Not stored; not retrievable again.
    pub token: String,
}

/// Issue a new token for `user`, named `name`.
pub async fn issue(pool: &PgPool, user: UserId, name: &str) -> Result<Issued, AccountError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AccountError::UnnamedToken);
    }
    let minted = secret::mint(secret::TOKEN_PREFIX)?;
    let mut tx = db::scoped(pool, user).await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO api_tokens (user_id, name, token_hash)
         VALUES (member_id(), $1, $2) RETURNING id",
    )
    .bind(name)
    .bind(minted.digest)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Issued {
        id,
        token: minted.plain,
    })
}

/// Whose token `bearer` is, if anybody's — and note that it was used.
///
/// Privileged for the same reason as [`super::sessions::find`].
pub async fn find(pool: &PgPool, bearer: &str) -> Result<Option<UserId>, AccountError> {
    let mut tx = db::privileged(pool).await?;
    let id: Option<i64> = sqlx::query_scalar(
        "UPDATE api_tokens SET last_used_at = now() WHERE token_hash = $1 RETURNING user_id",
    )
    .bind(secret::digest(bearer))
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id.map(UserId::from_row))
}

/// `user`'s tokens, newest first. No owner filter: the scope is the filter.
pub async fn list(pool: &PgPool, user: UserId) -> Result<Vec<TokenInfo>, AccountError> {
    let mut tx = db::scoped(pool, user).await?;
    let rows: Vec<(i64, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, name, extract(epoch FROM created_at)::bigint,
                extract(epoch FROM last_used_at)::bigint
         FROM api_tokens ORDER BY id DESC",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, created_at, last_used_at)| TokenInfo {
            id,
            name,
            created_at,
            last_used_at,
        })
        .collect())
}

/// Revoke `user`'s token `id`. `false` when they have no token by that id —
/// including when somebody else does, which is indistinguishable on purpose.
pub async fn revoke(pool: &PgPool, user: UserId, id: i64) -> Result<bool, AccountError> {
    let mut tx = db::scoped(pool, user).await?;
    let deleted = sqlx::query("DELETE FROM api_tokens WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;
    Ok(deleted == 1)
}
