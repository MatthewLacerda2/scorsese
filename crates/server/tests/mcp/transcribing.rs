//! `synth_import` and `synth_export` on the web (#678): a MIDI file is read
//! from the caller's library by id, and an export is kept there.

use scorsese_core::{AssetKind, hash_bytes};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, document, member};

/// How many MIDI files the library holds.
async fn midi_files(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM library_items WHERE kind = 'midi'")
        .fetch_one(pool)
        .await
        .expect("the library counts")
}

#[sqlx::test]
async fn a_library_midi_file_becomes_a_recipe_and_goes_back_out_to_the_library(pool: PgPool) {
    let files = common::files("mcp-midi");
    let storage = files.storage.clone();
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let sha = hash_bytes(common::TUNE);
    let home = storage.library_file(ana, &sha, "mid");
    std::fs::create_dir_all(home.parent().unwrap()).unwrap();
    std::fs::write(&home, common::TUNE).unwrap();
    let tune: i64 = sqlx::query_scalar(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, 'Rag.mid', 'midi', 'mid', 42, '{}') RETURNING id",
    )
    .bind(ana.get())
    .bind(&sha)
    .fetch_one(&pool)
    .await
    .unwrap();
    let video = common::hold(&pool, ana, &"b".repeat(64)).await;
    let id = super::stored(&pool, ana, json!({})).await;

    let wrong = json!({ "project": id, "item": video });
    let (said, refused) = call(address, &token, "synth_import", wrong).await;
    assert!(refused && said.contains("not a MIDI one"), "{said}");

    let (said, refused) = call(
        address,
        &token,
        "synth_import",
        json!({ "project": id, "item": tune }),
    )
    .await;
    assert!(!refused && said.contains("synth_audio, sketch"), "{said}");
    assert!(said.contains("2 notes") && !said.contains(&sha), "{said}");
    let stored = document(&pool, ana, id).await.document;
    let asset = stored
        .assets
        .iter()
        .find(|asset| asset.kind == AssetKind::SynthAudio);
    let asset = asset
        .expect("the import added a synth_audio asset")
        .id
        .to_string();
    assert!(
        asset.to_lowercase().starts_with("rag"),
        "named for the file: {asset}"
    );

    let out = json!({ "project": id, "asset": asset, "out": "x.mid" });
    let (said, refused) = call(address, &token, "synth_export", out).await;
    assert!(refused && said.contains("not taken"), "{said}");

    let export = json!({ "project": id, "asset": asset });
    let (said, refused) = call(address, &token, "synth_export", export.clone()).await;
    assert!(
        !refused && said.contains("Kept in your library as"),
        "{said}"
    );
    assert_eq!(midi_files(&pool).await, 2);
    let (again, refused) = call(address, &token, "synth_export", export).await;
    assert!(
        !refused && again.contains("already has exactly this file"),
        "{again}"
    );
    assert_eq!(midi_files(&pool).await, 2, "the same notes are kept once");

    let (listed, _) = call(address, &token, "library", json!({ "kind": "midi" })).await;
    assert_eq!(listed.lines().count(), 2, "{listed}");
}

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
    assert!(refused && said.contains("synth_import"), "{said}");
    assert!(document(&pool, ana, id).await.document.assets.is_empty());
}
