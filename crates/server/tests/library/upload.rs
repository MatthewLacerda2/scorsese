//! A file arrives over tus, in pieces, and is checked before it is kept.

use scorsese_core::hash_bytes;
use sqlx::postgres::PgPool;

use crate::common::{self, request, send};
use crate::{announce, announce_as, member, patch, upload, video};

#[sqlx::test]
async fn a_file_arrives_in_chunks_and_resumes_where_it_stopped(pool: PgPool) {
    let files = common::files("upload-chunks");
    let bytes = video(&common::scratch("upload-chunks-media"));
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (_, ana) = member(&pool, "ana@example.com").await;

    let created = announce(address, &ana, "Intro.MP4", &bytes).await;
    assert_eq!(created.status, 201, "{}", created.body);
    assert_eq!(created.header("tus-resumable"), Some("1.0.0"));
    let location = created.header("location").unwrap().to_owned();
    let head = |ana: String, location: String| async move {
        let headers = [ana.as_str(), "Tus-Resumable: 1.0.0"];
        send(address, "HEAD", &location, &headers, &[]).await
    };
    let fresh = head(ana.clone(), location.clone()).await;
    assert_eq!(fresh.header("upload-offset"), Some("0"));
    assert_eq!(
        fresh.header("upload-length"),
        Some(&*bytes.len().to_string())
    );

    // A chunk that does not start where the upload got to changes nothing.
    let misplaced = patch(address, &ana, &location, 10, &bytes[..20]).await;
    assert_eq!(misplaced.status, 409, "{}", misplaced.body);
    assert_eq!(misplaced.json()["offset"], 0);

    let half = bytes.len() / 2;
    let first = patch(address, &ana, &location, 0, &bytes[..half]).await;
    assert_eq!(first.header("upload-offset"), Some(&*half.to_string()));
    let resumed = head(ana.clone(), location.clone()).await;
    assert_eq!(resumed.header("upload-offset"), Some(&*half.to_string()));
    let last = patch(address, &ana, &location, half, &bytes[half..]).await;
    assert_eq!(last.status, 204, "{}", last.body);
    let id = last.header("scorsese-library-item").unwrap().to_owned();
    assert_eq!(
        head(ana.clone(), location).await.status,
        404,
        "the upload is done"
    );

    let listed = request(address, "GET", "/api/library", &[&ana], None).await;
    let tile = &listed.json()[0];
    assert_eq!(tile["id"].to_string(), id);
    assert_eq!(tile["name"], "Intro.MP4");
    assert_eq!(tile["kind"], "video");
    assert_eq!(tile["size_bytes"], bytes.len());
    assert_eq!(tile["thumbnail"], format!("/api/library/{id}/thumbnail"));
    assert!(
        tile.get("sha256").is_none(),
        "a list carries a tile, no more"
    );

    let path = format!("/api/library/{id}");
    let details = request(address, "GET", &path, &[&ana], None).await.json();
    assert_eq!(details["sha256"], hash_bytes(&bytes));
    assert_eq!(details["extension"], "mp4");
    assert_eq!(details["media"]["width"], 64);
    assert_eq!(details["generated"], false);
}

#[sqlx::test]
async fn the_same_bytes_again_are_refused_by_the_name_they_have(pool: PgPool) {
    let bytes = video(&common::scratch("upload-duplicate-media"));
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-dup")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "intro.mp4", &bytes).await;

    // What the browser asks before sending anything.
    let asked = format!("/api/library?sha256={}", hash_bytes(&bytes));
    let found = request(address, "GET", &asked, &[&ana], None).await.json();
    assert_eq!(found[0]["id"], id);

    let again = announce(address, &ana, "renamed copy.mov", &bytes).await;
    assert_eq!(again.status, 409, "{}", again.body);
    assert_eq!(again.json()["item"], id);
    assert_eq!(
        again.json()["error"],
        "you already have this as \u{201c}intro.mp4\u{201d}"
    );

    // Somebody else's library is not consulted: bia may upload the same file.
    let (_, bia) = member(&pool, "bia@example.com").await;
    assert_eq!(
        announce(address, &bia, "mine.mp4", &bytes).await.status,
        201
    );
}

#[sqlx::test]
async fn what_arrives_is_checked_rather_than_trusted(pool: PgPool) {
    let bytes = video(&common::scratch("upload-checked-media"));
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-checked")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;

    let options = send(address, "OPTIONS", "/api/uploads", &[], &[]).await;
    assert_eq!(options.header("tus-version"), Some("1.0.0"));
    assert_eq!(
        options.header("tus-extension"),
        Some("creation,termination")
    );

    let untus = send(address, "POST", "/api/uploads", &[&ana], &[]).await;
    assert_eq!(untus.status, 412);
    let notes = announce(address, &ana, "notes.txt", b"hello").await;
    assert_eq!(notes.status, 415, "{}", notes.body);

    // The bytes do not have the hash announced.
    let wrong = announce_as(address, &ana, "a.mp4", &"0".repeat(64), bytes.len()).await;
    let location = wrong.header("location").unwrap().to_owned();
    let refused = patch(address, &ana, &location, 0, &bytes).await;
    assert_eq!(refused.status, 422, "{}", refused.body);
    assert!(refused.body.contains("hash differs"), "{}", refused.body);

    // A .mp4 that is not a video at all.
    let junk = b"not a video, whatever its name says".to_vec();
    let created = announce(address, &ana, "fake.mp4", &junk).await;
    let location = created.header("location").unwrap().to_owned();
    let refused = patch(address, &ana, &location, 0, &junk).await;
    assert_eq!(refused.status, 422, "{}", refused.body);
    assert!(
        !refused.body.contains("/users/"),
        "no server path: {}",
        refused.body
    );

    let listed = request(address, "GET", "/api/library", &[&ana], None).await;
    assert_eq!(
        listed.json(),
        serde_json::json!([]),
        "nothing refused was kept"
    );
}
