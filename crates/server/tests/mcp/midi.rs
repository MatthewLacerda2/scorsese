//! A MIDI file in the library on the web: uploaded before #785 took
//! `synth_import` and `synth_export` off the tool list, it stays a library
//! file the user can list and delete, and is never imported as media.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, document, member};

#[sqlx::test]
async fn a_midi_file_is_not_imported_as_media(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let tune: i64 = sqlx::query_scalar(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, 'Rag.mid', 'midi', 'mid', 42, '{}') RETURNING id",
    )
    .bind(ana.get())
    .bind("c".repeat(64))
    .fetch_one(&pool)
    .await
    .unwrap();
    let id = super::stored(&pool, ana, json!({})).await;
    let bring = json!({ "project": id, "items": [tune] });
    let (said, refused) = call(address, &token, "import", bring).await;
    assert!(refused && said.contains("MIDI"), "{said}");
    assert!(document(&pool, ana, id).await.document.assets.is_empty());
}
