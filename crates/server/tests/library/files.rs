//! Opening a file: whole or in ranges, its thumbnail drawn by the worker, and
//! nothing of it reachable by another user.

use std::time::{Duration, Instant};

use scorsese_server::jobs::{self, kinds};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use crate::common::{self, request, send};
use crate::{member, patch, upload, video};

#[sqlx::test]
async fn a_file_is_served_whole_or_in_the_range_asked_for(pool: PgPool) {
    let bytes = video(&common::scratch("files-ranges-media"));
    let (address, _) = common::serve_with(pool.clone(), common::files("files-ranges")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "clip.mp4", &bytes).await;
    let path = format!("/api/library/{id}/file");
    let get = |range: Option<String>| {
        let (ana, path) = (ana.clone(), path.clone());
        async move {
            let mut headers = vec![ana];
            headers.extend(range.map(|range| format!("Range: bytes={range}")));
            let headers: Vec<&str> = headers.iter().map(String::as_str).collect();
            send(address, "GET", &path, &headers, &[]).await
        }
    };
    let size = bytes.len();

    let whole = get(None).await;
    assert_eq!(
        (whole.status, whole.bytes.as_slice()),
        (200, bytes.as_slice())
    );
    assert_eq!(whole.header("content-type"), Some("video/mp4"));
    assert_eq!(whole.header("accept-ranges"), Some("bytes"));

    let start = get(Some("0-9".into())).await;
    assert_eq!((start.status, start.bytes.as_slice()), (206, &bytes[..10]));
    assert_eq!(
        start.header("content-range"),
        Some(&*format!("bytes 0-9/{size}"))
    );
    let tail = get(Some("-5".into())).await;
    assert_eq!(tail.bytes, &bytes[size - 5..]);
    let rest = get(Some(format!("{}-", size - 3))).await;
    assert_eq!(
        (rest.status, rest.bytes.as_slice()),
        (206, &bytes[size - 3..])
    );
    let past = get(Some(format!("{size}-"))).await;
    assert_eq!(past.status, 416);
    assert_eq!(
        past.header("content-range"),
        Some(&*format!("bytes */{size}"))
    );
}

#[sqlx::test]
async fn the_worker_draws_each_files_thumbnail(pool: PgPool) {
    let files = common::files("files-thumbnail");
    let registry = kinds::registry(&files);
    let bytes = video(&common::scratch("files-thumbnail-media"));
    let (address, state) = common::serve_with(pool.clone(), files).await;
    let (_stop, stopping) = watch::channel(false);
    tokio::spawn(jobs::work(
        state.pool.clone(),
        registry,
        state.jobs.clone(),
        stopping,
    ));
    let (_, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "clip.mp4", &bytes).await;

    let path = format!("/api/library/{id}/thumbnail");
    let patience = Instant::now() + Duration::from_secs(30);
    let drawn = loop {
        let asked = request(address, "GET", &path, &[&ana], None).await;
        if asked.status == 200 || Instant::now() > patience {
            break asked;
        }
        assert_eq!(asked.json()["pending"], true, "{}", asked.body);
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(drawn.status, 200, "{}", drawn.body);
    assert_eq!(drawn.header("content-type"), Some("image/jpeg"));
    assert_eq!(&drawn.bytes[..2], b"\xff\xd8", "a JPEG");
}

#[sqlx::test]
async fn another_users_file_and_upload_are_not_there_for_them(pool: PgPool) {
    let bytes = video(&common::scratch("files-isolation-media"));
    let (address, _) = common::serve_with(pool.clone(), common::files("files-iso")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (_, bia) = member(&pool, "bia@example.com").await;
    let id = upload(address, &ana, "clip.mp4", &bytes).await;
    let pending = crate::announce(address, &ana, "b.mp4", b"somebytes").await;
    let location = pending.header("location").unwrap().to_owned();

    let listed = request(address, "GET", "/api/library", &[&bia], None).await;
    assert_eq!(listed.json(), serde_json::json!([]));
    let rename = serde_json::json!({ "name": "mine now" });
    for (method, path, body) in [
        ("GET", format!("/api/library/{id}"), None),
        ("GET", format!("/api/library/{id}/file"), None),
        ("GET", format!("/api/library/{id}/thumbnail"), None),
        ("PATCH", format!("/api/library/{id}"), Some(&rename)),
        ("DELETE", format!("/api/library/{id}"), None),
    ] {
        let answer = request(address, method, &path, &[&bia], body).await;
        assert_eq!(answer.status, 404, "{method} {path}: {}", answer.body);
    }
    assert_eq!(
        patch(address, &bia, &location, 0, b"somebytes")
            .await
            .status,
        404
    );
    let tus = [bia.as_str(), "Tus-Resumable: 1.0.0"];
    assert_eq!(
        send(address, "DELETE", &location, &tus, &[]).await.status,
        404
    );

    let anonymous = request(address, "GET", "/api/library", &[], None).await;
    assert_eq!(anonymous.status, 401);
}
