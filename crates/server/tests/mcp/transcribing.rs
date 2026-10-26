//! `transcribe` and `transcript_fix` on the web (#1549): a free job hears a
//! library recording through the Whisper the vendors name, its words kept on
//! the item, read back, corrected, and laid out beside the asset for the
//! registry's tools — `caption_narration` captions the footage.

use std::sync::atomic::Ordering;

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::vendors::{Mock, made, serve, settled};
use super::{call, document, member};

/// Two seconds of footage with sound, held in `user`'s library, and a new
/// project: the project's id and the file's hash.
async fn footage(
    pool: &PgPool,
    state: &scorsese_server::http::AppState,
    user: scorsese_server::db::UserId,
) -> (i64, String) {
    let bytes = made(
        "take.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x64:d=2:r=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=300:duration=2",
            "-pix_fmt",
            "yuv420p",
            "-shortest",
        ],
    );
    let sha = scorsese_core::hash_bytes(&bytes);
    let file = state.library.storage().library_file(user, &sha, "mp4");
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("the library folder");
    std::fs::write(&file, &bytes).expect("the file is kept");
    super::common::hold(pool, user, &sha).await;
    let id = super::stored(pool, user, json!({})).await;
    sqlx::query("UPDATE library_items SET media = $2 WHERE sha256 = $1")
        .bind(&sha)
        .bind(json!({ "duration_seconds": 2.0, "width": 64, "height": 64 }))
        .execute(pool)
        .await
        .expect("probed");
    (id, sha)
}

#[sqlx::test]
async fn footage_is_heard_by_a_free_job_then_read_back_corrected_and_captioned(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (id, sha) = footage(&pool, &state, ana).await;
    let item: i64 = sqlx::query_scalar("SELECT id FROM library_items WHERE sha256 = $1")
        .bind(&sha)
        .fetch_one(&pool)
        .await
        .expect("held");
    let (said, refused) = call(
        address,
        &who,
        "import",
        json!({ "project": id, "items": [item] }),
    )
    .await;
    assert!(!refused, "{said}");
    let asset = document(&pool, ana, id).await.document.assets[0]
        .id
        .to_string();

    let ask = json!({ "project": id, "asset": asset, "language": "pt" });
    let (said, refused) = call(address, &who, "transcribe", ask.clone()).await;
    assert!(!refused, "{said}");
    assert!(
        said.starts_with(&format!("Transcribing `{asset}`: job ")),
        "{said}"
    );
    assert!(said.contains("(transcribe)"), "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);
    assert_eq!(mock.heard.load(Ordering::SeqCst), 1);
    let kept: Value = sqlx::query_scalar("SELECT words FROM library_items WHERE id = $1")
        .bind(item)
        .fetch_one(&pool)
        .await
        .expect("the row");
    assert_eq!(kept["words"][0]["text"], "Bom", "{kept}");

    // Asked again, the kept words are read back, and nothing is heard.
    let (said, refused) = call(address, &who, "transcribe", ask).await;
    assert!(!refused, "{said}");
    assert!(said.contains("was already transcribed: 2 words"), "{said}");
    assert!(said.contains("   2  0.80–1.20  dia."), "{said}");
    assert_eq!(mock.heard.load(Ordering::SeqCst), 1);

    let fixes = json!({ "project": id, "asset": asset, "fixes": [{ "word": 2, "text": "dia!" }] });
    let (said, refused) = call(address, &who, "transcript_fix", fixes).await;
    assert!(!refused, "{said}");
    assert!(said.contains("1 fix, 0 words dropped"), "{said}");

    // The registry's captioning reads them beside the footage.
    let mut edited = serde_json::to_value(&document(&pool, ana, id).await.document).expect("json");
    edited["tracks"] = json!([{ "id": "v1", "kind": "video",
        "clips": [{ "id": "take", "asset": asset, "start": 0, "duration": 60 }] }]);
    let stored = document(&pool, ana, id).await;
    let edited = scorsese_core::Project::from_json(&edited.to_string()).expect("a project");
    scorsese_server::projects::save(&pool, ana, id, stored.summary.revision, &edited)
        .await
        .expect("placed");
    let (said, refused) = call(address, &who, "caption_narration", json!({ "project": id })).await;
    assert!(!refused, "{said}");
    assert!(said.starts_with("1 caption(s) from 1 line(s)"), "{said}");
    let captioned = document(&pool, ana, id).await.document;
    let caption = captioned
        .assets
        .iter()
        .find(|asset| asset.id.as_str() == "caption-take-1")
        .expect("the caption");
    assert_eq!(caption.text.as_deref(), Some("Bom dia!"));
}

#[sqlx::test]
async fn what_is_not_a_recording_is_refused_and_queues_nothing(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(
        &pool,
        ana,
        json!({ "assets": [{ "id": "title", "kind": "text", "text": "Hi" }] }),
    )
    .await;
    for (asked, says) in [
        (
            json!({ "project": id, "asset": "title" }),
            "`title` is text",
        ),
        (
            json!({ "project": id, "asset": "nope" }),
            "there is no asset `nope`",
        ),
        (json!({ "project": id }), "`asset` is required"),
        (
            json!({ "project": id, "asset": "title", "language": "Portuguese" }),
            "like `pt`",
        ),
    ] {
        let (said, refused) = call(address, &who, "transcribe", asked).await;
        assert!(refused, "{said}");
        assert!(said.contains(says), "{says} not in {said}");
    }
    let fix = json!({ "project": id, "asset": "title", "fixes": [{ "word": 1, "text": "x" }] });
    let (said, refused) = call(address, &who, "transcript_fix", fix).await;
    assert!(refused && said.contains("`title` is text"), "{said}");
    assert!(settled(&state, ana).await.is_empty());
}
