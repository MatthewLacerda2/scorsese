//! A browser's life over HTTP once it has a session: be somebody, log out.
//! How it gets one is `tests/google`'s.

mod common;

use common::request;
use scorsese_server::accounts::{sessions, users};
use sqlx::postgres::PgPool;

#[sqlx::test]
async fn a_session_lasts_until_logout(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    let opened = sessions::open(&pool, ana).await.unwrap();
    let cookie = format!("Cookie: scorsese_session={opened}");

    let me = request(address, "GET", "/api/me", &[cookie.as_str()], None).await;
    assert_eq!(me.status, 200);
    assert_eq!(me.json()["email"], "ana@example.com");

    let out = request(address, "POST", "/api/logout", &[cookie.as_str()], None).await;
    assert_eq!(out.status, 204);
    // The session and its hint (#1246) both go.
    let cleared = out.headers_named("set-cookie");
    for name in ["scorsese_session=;", "scorsese_signed_in=;"] {
        assert!(
            cleared
                .iter()
                .any(|c| c.starts_with(name) && c.contains("Max-Age=0")),
            "{name} {cleared:?}"
        );
    }
    let after = request(address, "GET", "/api/me", &[cookie.as_str()], None).await;
    assert_eq!(after.status, 401);
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
async fn the_password_routes_are_gone(pool: PgPool) {
    let address = common::serve(pool).await;
    for path in ["/api/login", "/api/me/password"] {
        let response = request(address, "POST", path, &[], None).await;
        assert!(
            matches!(response.status, 404 | 405),
            "{path}: {}",
            response.status
        );
    }
}
