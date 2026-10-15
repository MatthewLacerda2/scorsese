//! The access record (#1267, `accounts::access`): a row per account per day it
//! is used, with the visitor's address, kept past the account's deletion and
//! read by no member.

mod common;

use common::request;
use scorsese_server::accounts::access::{self, Access};
use scorsese_server::accounts::{sessions, tokens, users};
use scorsese_server::db;
use scorsese_server::storage::Storage;
use sqlx::postgres::PgPool;

/// Every row, oldest first: `(user_id, kind, address)`.
async fn rows(pool: &PgPool) -> Vec<(i64, String, Option<String>)> {
    sqlx::query_as("SELECT user_id, kind, host(address) FROM access_log ORDER BY id")
        .fetch_all(pool)
        .await
        .expect("the record reads")
}

#[sqlx::test]
async fn the_first_request_of_the_day_is_recorded_once_per_account(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    let bia = users::create(&pool, "bia@example.com").await.unwrap();
    let cookie = sessions::open(&pool, ana).await.unwrap();
    let cookie = format!("Cookie: scorsese_session={cookie}");
    let token = tokens::issue(&pool, bia, "laptop").await.unwrap();
    let bearer = format!("Authorization: Bearer {}", token.token);

    // nginx names the visitor; the header is what is kept, not the peer.
    let named = "X-Scorsese-Client: 2001:db8::7";
    for headers in [[cookie.as_str(), named], [cookie.as_str(), named]] {
        let me = request(address, "GET", "/api/me", &headers, None).await;
        assert_eq!(me.status, 200, "{}", me.body);
    }
    // A token is an access too; without the header, the connection's peer.
    let me = request(address, "GET", "/api/me", &[bearer.as_str()], None).await;
    assert_eq!(me.status, 200, "{}", me.body);
    // A request nobody is signed in to records nothing.
    let anon = request(address, "GET", "/api/me", &[named], None).await;
    assert_eq!(anon.status, 401);

    let day = || "day".to_owned();
    assert_eq!(
        rows(&pool).await,
        [
            (ana.get(), day(), Some("2001:db8::7".to_owned())),
            (bia.get(), day(), Some("127.0.0.1".to_owned())),
        ]
    );
}

#[sqlx::test]
async fn a_day_is_one_row_however_often_it_is_written(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    let here = Some("203.0.113.9".parse().unwrap());
    // As after a restart, when the server's memory of today is gone.
    for _ in 0..2 {
        access::record(&pool, ana, here, Access::Day).await.unwrap();
    }
    // Sign-ins are each their own row.
    for _ in 0..2 {
        access::record(&pool, ana, here, Access::SignIn)
            .await
            .unwrap();
    }
    access::record(&pool, ana, None, Access::Day).await.unwrap();
    let kinds: Vec<String> = rows(&pool).await.into_iter().map(|row| row.1).collect();
    assert_eq!(kinds, ["day", "sign_in", "sign_in"]);
}

#[sqlx::test]
async fn the_record_outlives_the_account(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    access::record(&pool, ana, None, Access::SignIn)
        .await
        .unwrap();
    let root = common::scratch::dir("1267-delete");
    let files = Storage::new(root.join("kept"), root.join("cache"));
    users::delete(&pool, &files, "ana@example.com")
        .await
        .unwrap();
    assert_eq!(rows(&pool).await, [(ana.get(), "sign_in".to_owned(), None)]);
}

#[sqlx::test]
async fn no_member_reads_the_record_not_even_their_own_rows(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    access::record(&pool, ana, None, Access::SignIn)
        .await
        .unwrap();
    let members = db::member_pool(&pool).await.unwrap();
    let mut tx = db::scoped(&members, ana).await.unwrap();
    let error = sqlx::query("SELECT count(*) FROM access_log")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("permission denied"), "{error}");
}
