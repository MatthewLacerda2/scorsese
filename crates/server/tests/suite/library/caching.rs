//! A user's file is never reused from a browser's cache without asking: each
//! use is an authorized request, answered `304` when the copy held is still
//! the file, so logging out leaves nothing private that opens on its own.

use sqlx::postgres::PgPool;

use crate::common::{self, send};
use crate::library::{member, upload, video};

#[sqlx::test]
async fn a_file_is_revalidated_by_its_tag_and_never_kept_without_asking(pool: PgPool) {
    let bytes = video(&common::scratch::dir("caching-media"));
    let (address, _) = common::serve_with(pool.clone(), common::files("caching")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (_, bia) = member(&pool, "bia@example.com").await;
    let id = upload(address, &ana, "clip.mp4", &bytes).await;
    let path = format!("/api/library/{id}/file");

    let whole = send(address, "GET", &path, &[&ana], &[]).await;
    assert_eq!(whole.status, 200);
    assert_eq!(whole.header("cache-control"), Some("private, no-cache"));
    let tag = whole.header("etag").expect("a tag").to_owned();
    let asking = format!("If-None-Match: {tag}");

    let same = send(address, "GET", &path, &[&ana, &asking], &[]).await;
    assert_eq!((same.status, same.bytes.len()), (304, 0));
    assert_eq!(same.header("etag"), Some(tag.as_str()));
    assert_eq!(same.header("cache-control"), Some("private, no-cache"));
    let ranged = [ana.as_str(), &asking, "Range: bytes=0-9"];
    let part = send(address, "GET", &path, &ranged, &[]).await;
    assert_eq!(part.status, 304, "the tag is asked about before the range");

    let stale = send(address, "GET", &path, &[&ana, "If-None-Match: \"x\""], &[]).await;
    assert_eq!((stale.status, stale.bytes.len()), (200, bytes.len()));
    let any = send(address, "GET", &path, &[&ana, "If-None-Match: *"], &[]).await;
    assert_eq!(any.status, 304);

    let signed_out = send(address, "GET", &path, &[&asking], &[]).await;
    assert_eq!(signed_out.status, 401, "a tag is no credential");
    let theirs = send(address, "GET", &path, &[&bia, &asking], &[]).await;
    assert_eq!(theirs.status, 404, "nor does it reach another user's file");
}
