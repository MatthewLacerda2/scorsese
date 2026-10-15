//! A web page is a library kind (#1144), as `scorsese import` takes one: text,
//! never probed, drawn no thumbnail, and never served as a page that runs.

use sqlx::postgres::PgPool;

use crate::common::{self, request, send};
use crate::{announce, member, upload};

const PAGE: &[u8] = b"<!doctype html><title>Card</title><p>Hello</p>";

#[sqlx::test]
async fn a_page_is_kept_as_text(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-page")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "Card.html", PAGE).await;

    let listed = request(address, "GET", "/api/library", &[&ana], None).await;
    assert_eq!(listed.json()[0]["kind"], "html", "{}", listed.body);
    let thumbnail = format!("/api/library/{id}/thumbnail");
    let none = request(address, "GET", &thumbnail, &[&ana], None).await;
    assert_eq!(none.status, 404, "{}", none.body);
    assert_eq!(none.json()["pending"], false, "{}", none.body);

    let served = send(
        address,
        "GET",
        &format!("/api/library/{id}/file"),
        &[&ana],
        &[],
    )
    .await;
    assert_eq!(served.status, 200, "{}", served.body);
    assert_eq!(
        served.header("content-type"),
        Some("text/plain; charset=utf-8"),
        "a page is never run from the app's origin"
    );
}

#[sqlx::test]
async fn a_page_that_is_not_text_or_too_large_is_refused(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("refuse-page")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let large = vec![b'a'; (1 << 20) + 1];
    let refused = announce(address, &ana, "huge.html", &large).await;
    assert_eq!(refused.status, 422, "{}", refused.body);

    let binary: &[u8] = &[0xff, 0xfe, 0x00, 0x80];
    let created = announce(address, &ana, "odd.html", binary).await;
    assert_eq!(created.status, 201, "{}", created.body);
    let location = created.header("location").expect("a Location").to_owned();
    let last = crate::patch(address, &ana, &location, 0, binary).await;
    assert_eq!(last.status, 422, "{}", last.body);
    assert!(last.body.contains("not a web page"), "{}", last.body);
}
