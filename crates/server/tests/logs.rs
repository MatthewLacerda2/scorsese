//! The server's log (#1520): a request writes one line naming its matched
//! route, never its raw path, the account asking, its status and how long it
//! took; the health check writes none.

mod common;

use scorsese_server::accounts::{tokens, users};
use sqlx::postgres::PgPool;

#[sqlx::test]
async fn a_request_is_logged_by_route_account_status_and_time(pool: PgPool) {
    let (_logging, kept) = common::logged::keep();
    let address = common::serve(pool.clone()).await;
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    let token = tokens::issue(&pool, ana, "script").await.unwrap();

    let bearer = format!("Authorization: Bearer {}", token.token);
    let missing = common::request(address, "GET", "/api/jobs/987654", &[&bearer], None).await;
    assert_eq!(missing.status, 404, "{}", missing.body);
    let (healthy, _) = common::get(address, "/api/health").await;
    assert_eq!(healthy, 200);

    let log = kept.text();
    let requests: Vec<&str> = log
        .lines()
        .filter(|line| line.contains("request{"))
        .collect();
    let [line] = &requests[..] else {
        panic!("one line, for the job and not the health check:\n{log}");
    };
    for field in [
        "method=GET",
        "route=/api/jobs/{id}",
        &format!("user={}", ana.get()),
        "status=404",
        "ms=",
    ] {
        assert!(line.contains(field), "{field} in {line}");
    }
    assert!(!line.contains("987654"), "the raw path carries ids: {line}");
    assert!(!line.contains("ana@example.com"), "{line}");
}
