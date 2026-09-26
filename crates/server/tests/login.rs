//! A browser's life over HTTP: log in, be somebody, change the password, log
//! out.

mod common;

use std::net::SocketAddr;

use common::{Response, request};
use scorsese_server::accounts::users;
use serde_json::json;
use sqlx::postgres::PgPool;

async fn login(address: SocketAddr, email: &str, password: &str) -> Response {
    let body = json!({ "email": email, "password": password });
    request(address, "POST", "/api/login", &[], Some(&body)).await
}

/// The `Cookie:` header line a browser would send back after `response`.
fn cookie_from(response: &Response) -> String {
    let set = response.header("set-cookie").expect("a cookie was set");
    let pair = set.split(';').next().expect("a cookie has a name=value");
    format!("Cookie: {pair}")
}

#[sqlx::test]
async fn a_session_lasts_until_logout(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    users::create(&pool, "ana@example.com", "correct horse")
        .await
        .unwrap();

    let response = login(address, "Ana@Example.com", "correct horse").await;
    assert_eq!(response.status, 200, "{}", response.body);
    assert_eq!(response.json()["email"], "ana@example.com");
    let set = response.header("set-cookie").unwrap();
    for attribute in [
        "HttpOnly",
        "Secure",
        "SameSite=Strict",
        "Path=/api",
        "Max-Age=2592000",
    ] {
        assert!(set.contains(attribute), "{set}");
    }
    let cookie = cookie_from(&response);

    let me = request(address, "GET", "/api/me", &[cookie.as_str()], None).await;
    assert_eq!(me.status, 200);
    assert_eq!(me.json()["email"], "ana@example.com");

    let out = request(address, "POST", "/api/logout", &[cookie.as_str()], None).await;
    assert_eq!(out.status, 204);
    assert!(out.header("set-cookie").unwrap().contains("Max-Age=0"));
    let after = request(address, "GET", "/api/me", &[cookie.as_str()], None).await;
    assert_eq!(after.status, 401);
}

#[sqlx::test]
async fn a_wrong_password_and_an_unknown_email_read_the_same(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    users::create(&pool, "ana@example.com", "correct horse")
        .await
        .unwrap();

    let wrong = login(address, "ana@example.com", "wrong horse").await;
    let unknown = login(address, "bia@example.com", "correct horse").await;
    assert_eq!((wrong.status, unknown.status), (401, 401));
    assert_eq!(wrong.body, unknown.body);
    assert!(wrong.header("set-cookie").is_none());
}

#[sqlx::test]
async fn nothing_per_user_answers_without_credentials(pool: PgPool) {
    let address = common::serve(pool).await;
    let forged = "Cookie: scorsese_session=0000";
    for headers in [vec![], vec![forged]] {
        for (method, path) in [
            ("GET", "/api/me"),
            ("GET", "/api/tokens"),
            ("POST", "/api/logout"),
        ] {
            let response = request(address, method, path, &headers, None).await;
            assert_eq!(response.status, 401, "{method} {path} {headers:?}");
            assert!(response.json()["error"].is_string());
        }
    }
}

#[sqlx::test]
async fn a_user_changes_their_own_password(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    users::create(&pool, "ana@example.com", "given by operator")
        .await
        .unwrap();
    let cookie = cookie_from(&login(address, "ana@example.com", "given by operator").await);

    let change = |current: &'static str| json!({ "current": current, "new": "chosen by ana" });
    let path = "/api/me/password";
    let wrong = request(
        address,
        "POST",
        path,
        &[cookie.as_str()],
        Some(&change("guess")),
    )
    .await;
    assert_eq!(wrong.status, 400);
    let right = request(
        address,
        "POST",
        path,
        &[cookie.as_str()],
        Some(&change("given by operator")),
    )
    .await;
    assert_eq!(right.status, 204, "{}", right.body);

    assert_eq!(
        login(address, "ana@example.com", "chosen by ana")
            .await
            .status,
        200
    );
}
