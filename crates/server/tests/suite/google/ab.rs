//! A Google sign-in links the landing's visitor code to the account (#1553).

use sqlx::postgres::PgPool;

use super::common::request;
use super::flow::{query, start};
use super::{Recorded, serve};

const CODE: &str = "0b7f6a3e-9c1d-4e2f-8a5b-6c7d8e9f0a1b";

#[sqlx::test]
async fn a_sign_in_links_the_browsers_visitor_to_the_account(pool: PgPool) {
    let google = Recorded::answering("1101", "ana@example.com");
    let address = serve(pool.clone(), Some(google)).await;
    let visit = serde_json::json!({ "visitor": CODE, "events": [{ "kind": "view" }] });
    let posted = request(address, "POST", "/api/ab/events", &[], Some(&visit)).await;
    assert_eq!(posted.status, 204);

    let (url, flow) = start(address, "%2Fprojects").await;
    let path = format!(
        "/api/auth/google/callback?code=4%2F0Ab&state={}",
        query(&url, "state")
    );
    let cookie = format!(
        "Cookie: {}; scorsese_visitor={CODE}",
        flow.split(';').next().unwrap()
    );
    let signed = request(address, "GET", &path, &[cookie.as_str()], None).await;
    assert_eq!(signed.status, 303, "{}", signed.body);
    assert_eq!(signed.header("location"), Some("/projects"));

    let linked: Option<String> = sqlx::query_scalar(
        "SELECT u.email FROM ab_visitors v JOIN users u ON u.id = v.user_id
         WHERE v.code = $1::uuid AND v.linked_at IS NOT NULL",
    )
    .bind(CODE)
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(linked.as_deref(), Some("ana@example.com"));
}
