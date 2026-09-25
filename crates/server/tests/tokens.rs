//! API tokens over HTTP: issued from a session, used as a bearer, revoked —
//! and never reachable by another user.

mod common;

use std::net::SocketAddr;

use common::request;
use scorsese_server::accounts::{sessions, users};
use serde_json::json;
use sqlx::postgres::PgPool;

/// A logged-in browser's `Cookie:` line for a new account `email`.
async fn member(pool: &PgPool, email: &str) -> String {
    let user = users::create(pool, email, "password one")
        .await
        .expect("the account is created");
    let cookie = sessions::open(pool, user).await.expect("a session opens");
    format!("Cookie: scorsese_session={cookie}")
}

/// Issue a token named `name` with `credential`, returning `(id, token)`.
async fn issue(address: SocketAddr, credential: &str, name: &str) -> (i64, String) {
    let body = json!({ "name": name });
    let response = request(address, "POST", "/api/tokens", &[credential], Some(&body)).await;
    assert_eq!(response.status, 201, "{}", response.body);
    let issued = response.json();
    (
        issued["id"].as_i64().expect("an id"),
        issued["token"].as_str().expect("a token").to_owned(),
    )
}

#[sqlx::test]
async fn a_token_acts_as_its_user_until_revoked(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = member(&pool, "ana@example.com").await;
    let (id, token) = issue(address, &ana, "laptop").await;
    assert!(token.starts_with("scor_"), "{token}");
    let bearer = format!("Authorization: Bearer {token}");

    let me = request(address, "GET", "/api/me", &[bearer.as_str()], None).await;
    assert_eq!(me.json()["email"], "ana@example.com");

    let listed = request(address, "GET", "/api/tokens", &[ana.as_str()], None)
        .await
        .json();
    assert_eq!(listed[0]["name"], "laptop");
    assert!(listed[0]["last_used_at"].is_i64(), "{listed}");
    assert!(
        !listed.to_string().contains(&token),
        "a listing never shows a token"
    );

    // A token cannot mint another: revoking a leaked one has to be enough.
    let body = json!({ "name": "sneaky" });
    let minted = request(
        address,
        "POST",
        "/api/tokens",
        &[bearer.as_str()],
        Some(&body),
    )
    .await;
    assert_eq!(minted.status, 403);

    let revoked = request(
        address,
        "DELETE",
        &format!("/api/tokens/{id}"),
        &[ana.as_str()],
        None,
    )
    .await;
    assert_eq!(revoked.status, 204);
    assert_eq!(
        request(address, "GET", "/api/me", &[bearer.as_str()], None)
            .await
            .status,
        401
    );
}

#[sqlx::test]
async fn one_user_cannot_see_or_revoke_anothers_tokens(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = member(&pool, "ana@example.com").await;
    let bia = member(&pool, "bia@example.com").await;
    let (anas, _) = issue(address, &ana, "ana's").await;
    issue(address, &bia, "bia's").await;

    let seen = request(address, "GET", "/api/tokens", &[bia.as_str()], None)
        .await
        .json();
    assert_eq!(seen.as_array().unwrap().len(), 1);
    assert_eq!(seen[0]["name"], "bia's");

    let path = format!("/api/tokens/{anas}");
    let attempt = request(address, "DELETE", &path, &[bia.as_str()], None).await;
    assert_eq!(
        attempt.status, 404,
        "the same answer as for an id nobody has"
    );
    let still = request(address, "GET", "/api/tokens", &[ana.as_str()], None)
        .await
        .json();
    assert_eq!(still[0]["name"], "ana's");
}

#[sqlx::test]
async fn a_bad_bearer_is_refused_even_beside_a_good_cookie(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = member(&pool, "ana@example.com").await;
    for bearer in [
        "Authorization: Bearer scor_nope",
        "Authorization: Basic YTpi",
    ] {
        let response = request(address, "GET", "/api/me", &[ana.as_str(), bearer], None).await;
        assert_eq!(response.status, 401, "{bearer}");
    }
}
