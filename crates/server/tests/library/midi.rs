//! A MIDI file is a library file like any other (#678): uploaded, listed and
//! opened the same way — checked as MIDI rather than probed, and with no
//! thumbnail, since there is no picture of notes to draw.

use sqlx::postgres::PgPool;

use crate::common::{self, TUNE, request};
use crate::{announce, member, patch, upload};

#[sqlx::test]
async fn a_midi_file_is_kept_listed_and_served_without_a_picture(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-midi")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "Rag.MID", TUNE).await;

    let listed = request(address, "GET", "/api/library?kind=midi", &[&ana], None).await;
    let tile = &listed.json()[0];
    assert_eq!(tile["id"], id);
    assert_eq!(tile["kind"], "midi");

    let details = request(address, "GET", &format!("/api/library/{id}"), &[&ana], None).await;
    assert_eq!(details.json()["extension"], "mid", "{}", details.body);
    assert_eq!(details.json()["media"], serde_json::json!({}));

    let file = request(
        address,
        "GET",
        &format!("/api/library/{id}/file"),
        &[&ana],
        None,
    )
    .await;
    assert_eq!(file.status, 200, "{}", file.body);
    assert_eq!(file.header("content-type"), Some("audio/midi"));

    let picture = format!("/api/library/{id}/thumbnail");
    let thumbnail = request(address, "GET", &picture, &[&ana], None).await;
    assert_eq!(thumbnail.status, 404, "{}", thumbnail.body);
    let queued: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs WHERE kind = 'thumbnail'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        queued, 0,
        "nothing is drawn for a MIDI file, now or when asked"
    );
}

#[sqlx::test]
async fn a_mid_that_is_not_midi_is_refused_in_the_readers_words(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-not-midi")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let junk = b"not midi, whatever its name says".to_vec();
    let created = announce(address, &ana, "notes.mid", &junk).await;
    assert_eq!(created.status, 201, "{}", created.body);
    let location = created.header("location").unwrap().to_owned();
    let refused = patch(address, &ana, &location, 0, &junk).await;
    assert_eq!(refused.status, 422, "{}", refused.body);
    assert!(
        refused.body.contains("could not be read as MIDI"),
        "{}",
        refused.body
    );
    let listed = request(address, "GET", "/api/library", &[&ana], None).await;
    assert_eq!(listed.json(), serde_json::json!([]));
}
