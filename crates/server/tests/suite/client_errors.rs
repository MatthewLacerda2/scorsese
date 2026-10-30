//! `POST /api/client-errors` (#1428): a crash report is taken from anyone,
//! never echoed, refused when it is too heavy or too frequent.

use crate::common;

use common::{request, send};
use scorsese_server::http::client_errors::{CAP, REPORTS_PER_HOUR, REPORTS_PER_MINUTE};
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

#[sqlx::test]
async fn one_ipv6_network_is_counted_as_one_address(pool: PgPool) {
    let address = common::serve(pool).await;
    let report = json!({ "message": "again" });
    // One machine holds a /64 whole, and may send from any address in it.
    for host in 0..REPORTS_PER_MINUTE {
        let from = format!("X-Scorsese-Client: 2001:db8:1:2::{:x}", host + 1);
        let taken = request(address, "POST", PATH, &[&from], Some(&report)).await;
        assert_eq!(taken.status, 204, "{}", taken.body);
    }
    let from = "X-Scorsese-Client: 2001:db8:1:2:ffff::1";
    let refused = request(address, "POST", PATH, &[from], Some(&report)).await;
    assert_eq!(refused.status, 429, "{}", refused.body);

    let next = "X-Scorsese-Client: 2001:db8:1:3::1";
    let taken = request(address, "POST", PATH, &[next], Some(&report)).await;
    assert_eq!(taken.status, 204, "the next /64 is another network");
}

#[sqlx::test]
async fn everybody_together_is_held_to_an_hours_worth(pool: PgPool) {
    let address = common::serve(pool).await;
    let report = json!({ "message": "everywhere" });
    // Each address stays under its own minute's limit.
    let mut sent = 0;
    for network in 0..REPORTS_PER_HOUR.div_ceil(REPORTS_PER_MINUTE) {
        let from = format!("X-Scorsese-Client: 198.51.100.{network}");
        for _ in 0..REPORTS_PER_MINUTE.min(REPORTS_PER_HOUR - sent) {
            let taken = request(address, "POST", PATH, &[&from], Some(&report)).await;
            assert_eq!(taken.status, 204, "{}", taken.body);
            sent += 1;
        }
    }
    let fresh = "X-Scorsese-Client: 203.0.113.200";
    let refused = request(address, "POST", PATH, &[fresh], Some(&report)).await;
    assert_eq!(refused.status, 429, "{}", refused.body);
    assert!(refused.body.contains("everyone"), "{}", refused.body);
    assert!(refused.header("Retry-After").is_some());
}
