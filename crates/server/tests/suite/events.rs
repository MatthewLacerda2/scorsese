//! `GET /api/events` lasts only as long as the credential it was opened
//! with: logging out, the operator's `user logout` and revoking a token each
//! end a stream already open, not only the requests after it.

use std::net::SocketAddr;
use std::time::Duration;

use scorsese_server::accounts::{sessions, tokens, users};
use sqlx::postgres::PgPool;

use crate::common;
use common::request;
use common::stream::{ends, listen};

/// How often the server under test asks whether a stream's credential holds.
const RECHECK: Duration = Duration::from_millis(100);

/// Long enough for many rechecks, and so for a stream to have ended.
const PATIENCE: Duration = Duration::from_secs(5);

async fn serve(pool: &PgPool) -> SocketAddr {
    let files = common::files("events");
    let shape = |state: scorsese_server::http::AppState| state.with_recheck(RECHECK);
    common::serve_shaped(pool.clone(), files, shape).await.0
}

/// A `Cookie:` line for a new session of a new account `email`.
async fn signed_in(pool: &PgPool, email: &str) -> String {
    let user = users::create(pool, email).await.expect("an account");
    let session = sessions::open(pool, user).await.expect("a session");
    format!("Cookie: scorsese_session={session}")
}

#[sqlx::test]
async fn a_stream_ends_when_its_session_does(pool: PgPool) {
    let address = serve(&pool).await;
    let ana = signed_in(&pool, "ana@example.com").await;
    let bia = signed_in(&pool, "bia@example.com").await;
    let carla = signed_in(&pool, "carla@example.com").await;
    let mut ana_hears = listen(address, &ana).await;
    let mut bia_hears = listen(address, &bia).await;
    let mut carla_hears = listen(address, &carla).await;

    let out = request(address, "POST", "/api/logout", &[ana.as_str()], None).await;
    assert_eq!(out.status, 204);
    assert!(ends(&mut ana_hears, PATIENCE).await, "logging out ends it");

    let bia_id = users::find(&pool, "bia@example.com").await.unwrap();
    sessions::close_all(&pool, bia_id).await.unwrap();
    assert!(ends(&mut bia_hears, PATIENCE).await, "user logout ends it");

    let still = ends(&mut carla_hears, RECHECK * 5).await;
    assert!(!still, "a session that holds keeps its stream");
}

#[sqlx::test]
async fn a_stream_ends_when_its_token_is_revoked(pool: PgPool) {
    let address = serve(&pool).await;
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    let revoked = tokens::issue(&pool, ana, "laptop").await.unwrap();
    let kept = tokens::issue(&pool, ana, "desktop").await.unwrap();
    let bearer = |token: &str| format!("Authorization: Bearer {token}");
    let mut revoked_hears = listen(address, &bearer(&revoked.token)).await;
    let mut kept_hears = listen(address, &bearer(&kept.token)).await;

    assert!(tokens::revoke(&pool, ana, revoked.id).await.unwrap());
    assert!(ends(&mut revoked_hears, PATIENCE).await, "revoking ends it");
    let still = ends(&mut kept_hears, RECHECK * 5).await;
    assert!(!still, "a token that holds keeps its stream");
}
