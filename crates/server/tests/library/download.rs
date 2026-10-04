//! Downloading a file (#711): the same bytes as opening it, sent as an
//! attachment under the file's own name and real extension — and for nobody
//! but its owner. A MIDI file stands in for every kind: it is the one with
//! no picture, and the route does not look at the kind.

use sqlx::postgres::PgPool;

use crate::common::{self, TUNE, request};
use crate::{member, upload};

#[sqlx::test]
async fn a_file_downloads_under_its_name_for_its_owner_only(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("download")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (_, bia) = member(&pool, "bia@example.com").await;
    let id = upload(address, &ana, "Rag.MID", TUNE).await;
    let download = format!("/api/library/{id}/file?download=1");

    let saved = request(address, "GET", &download, &[&ana], None).await;
    assert_eq!(saved.status, 200, "{}", saved.body);
    assert_eq!(saved.bytes, TUNE);
    assert_eq!(saved.header("content-type"), Some("audio/midi"));
    assert_eq!(
        saved.header("content-disposition"),
        Some("attachment; filename=\"Rag.MID\"; filename*=UTF-8''Rag.MID"),
        "a name already ending in its extension keeps it, once"
    );

    let played = request(address, "GET", &format!("/api/library/{id}/file"), &[&ana], None).await;
    assert_eq!(played.header("content-disposition"), None, "playing is not saving");

    let rename = serde_json::json!({ "name": "Café \"rag\"" });
    let path = format!("/api/library/{id}");
    let renamed = request(address, "PATCH", &path, &[&ana], Some(&rename)).await;
    assert_eq!(renamed.status, 200, "{}", renamed.body);
    let saved = request(address, "GET", &download, &[&ana], None).await;
    assert_eq!(
        saved.header("content-disposition"),
        Some("attachment; filename=\"Caf_ _rag_.mid\"; filename*=UTF-8''Caf%C3%A9%20%22rag%22.mid"),
        "a renamed file saves under its new name, with its extension added"
    );

    let theirs = request(address, "GET", &download, &[&bia], None).await;
    assert_eq!(theirs.status, 404, "{}", theirs.body);
    assert_eq!(theirs.header("content-disposition"), None);
}
