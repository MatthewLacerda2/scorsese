//! A sign-in is written to the access record (#1267) with the visitor's
//! address, beside the day's first request.

use sqlx::postgres::PgPool;

use super::common::request;
use super::flow::{query, start};
use super::{Recorded, serve};

#[sqlx::test]
async fn a_sign_in_is_recorded_with_the_visitors_address(pool: PgPool) {
    let google = Recorded::answering("1101", "ana@example.com");
    let address = serve(pool.clone(), Some(google)).await;
    let (url, flow) = start(address, "%2Fprojects").await;
    let path = format!(
        "/api/auth/google/callback?code=4%2F0Ab&state={}",
        query(&url, "state")
    );
    let cookie = format!("Cookie: {}", flow.split(';').next().unwrap());
    let headers = [cookie.as_str(), "X-Scorsese-Client: 198.51.100.4"];
    let signed = request(address, "GET", &path, &headers, None).await;
    assert_eq!(signed.status, 303, "{}", signed.body);
    assert_eq!(signed.header("location"), Some("/projects"));

    let rows: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT kind, host(address) FROM access_log ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [("sign_in".to_owned(), Some("198.51.100.4".to_owned()))]
    );
}
