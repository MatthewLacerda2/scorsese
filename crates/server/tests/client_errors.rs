//! `POST /api/client-errors` (#1428): a crash report is taken from anyone,
//! never echoed, refused when it is too heavy or too frequent.

mod common;

use common::{request, send};
use scorsese_server::http::client_errors::{CAP, REPORTS_PER_MINUTE};
use serde_json::json;
use sqlx::postgres::PgPool;

const PATH: &str = "/api/client-errors";

#[sqlx::test]
async fn a_report_from_anyone_is_taken_and_never_echoed(pool: PgPool) {
    let address = common::serve(pool).await;
    let report = json!({
        "message": "busy.includes is not a function",
        "stack": "at Busy\nat ChatPanel",
        "component_stack": "\n    in Busy",
        "path": "/projects/7/edit",
        "language": "pt-BR",
        "build": "abc123",
    });
    let taken = request(address, "POST", PATH, &[], Some(&report)).await;
    assert_eq!(taken.status, 204, "{}", taken.body);
    assert!(taken.body.is_empty());

    // A message alone is enough: an error thrown as a string has no stack.
    let bare = request(address, "POST", PATH, &[], Some(&json!({ "message": "x" }))).await;
    assert_eq!(bare.status, 204, "{}", bare.body);

    // Garbage is refused in the route's own words, not the body's.
    let garbled = send(
        address,
        "POST",
        PATH,
        &["Content-Type: application/json"],
        b"{\"secret\":",
    )
    .await;
    assert_eq!(garbled.status, 400);
    assert!(!garbled.body.contains("secret"), "{}", garbled.body);
}

#[sqlx::test]
async fn an_oversized_report_is_refused(pool: PgPool) {
    let address = common::serve(pool).await;
    let heavy = json!({ "message": "x".repeat(CAP) });
    let refused = request(address, "POST", PATH, &[], Some(&heavy)).await;
    assert_eq!(refused.status, 413, "{}", refused.body);
    assert!(!refused.body.contains("xxxx"));
}

#[sqlx::test]
async fn one_address_reporting_in_a_loop_is_told_to_wait(pool: PgPool) {
    let address = common::serve(pool).await;
    let report = json!({ "message": "again" });
    let looping = "X-Scorsese-Client: 203.0.113.9";
    for _ in 0..REPORTS_PER_MINUTE {
        let taken = request(address, "POST", PATH, &[looping], Some(&report)).await;
        assert_eq!(taken.status, 204, "{}", taken.body);
    }
    let refused = request(address, "POST", PATH, &[looping], Some(&report)).await;
    assert_eq!(refused.status, 429);
    assert!(refused.header("Retry-After").is_some());

    // Another address is not held up by it.
    let other = "X-Scorsese-Client: 2001:db8::7";
    let taken = request(address, "POST", PATH, &[other], Some(&report)).await;
    assert_eq!(taken.status, 204, "{}", taken.body);
}
