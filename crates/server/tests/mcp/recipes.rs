//! Synthesis and the script on the web (#560): a recipe written by one call is
//! there for the next, a bake is kept in the library, and a script is kept
//! where the document says it is.

use scorsese_core::GenerationState;
use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, document, member};

#[sqlx::test]
async fn a_sound_is_started_read_and_baked_into_the_library(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, json!({})).await;

    let started = json!({ "project": id, "name": "theme" });
    let (said, refused) = call(address, &token, "synth_new", started).await;
    assert!(!refused && said.contains("recipes/theme.json"), "{said}");
    let (_, kept) = projects::open_with_files(&pool, ana, id).await.unwrap();
    let recipe = kept.get("recipes/theme.json").expect("the recipe is kept");

    let asked = json!({ "project": id, "recipe": "recipes/theme.json" });
    let (read, refused) = call(address, &token, "synth_read", asked).await;
    assert!(!refused && read.contains(recipe.trim()), "{read}");

    let (baked, refused) = call(address, &token, "synth_bake", json!({ "project": id })).await;
    assert!(!refused, "{baked}");
    let stored = document(&pool, ana, id).await;
    let asset = &stored.document.assets[0];
    assert_eq!(asset.state, Some(GenerationState::Generated));
    let sha256 = asset.sha256.clone().expect("the bake is hashed");
    let brief: Option<String> =
        sqlx::query_scalar("SELECT brief_hash FROM library_items WHERE sha256 = $1")
            .bind(&sha256)
            .fetch_one(&pool)
            .await
            .expect("the bake is in the library");
    let path = asset
        .path
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(path.contains(&brief.expect("as a generation")), "{path}");

    // Baked already: the library's file is linked in, and nothing changes.
    let (again, refused) = call(address, &token, "synth_bake", json!({ "project": id })).await;
    assert!(!refused, "{again}");
    let after = document(&pool, ana, id).await.summary.revision;
    assert_eq!(after, stored.summary.revision, "{again}");
}

#[sqlx::test]
async fn a_script_is_kept_and_read_back_where_the_project_keeps_one(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, json!({})).await;

    let script = "# Teaser\n\nNever claim it is the fastest.\n";
    let written = json!({ "project": id, "text": script });
    let (said, refused) = call(address, &token, "script_write", written).await;
    assert!(!refused, "{said}");
    let (read, refused) = call(address, &token, "script_read", json!({ "project": id })).await;
    assert!(!refused && read == script, "{read}");
    let (stored, kept) = projects::open_with_files(&pool, ana, id).await.unwrap();
    assert_eq!(
        stored
            .document
            .script
            .map(|path| path.to_string())
            .as_deref(),
        Some("script.md")
    );
    assert_eq!(kept.get("script.md"), Some(script));

    // Among the media nothing is kept, so a script there is refused whole.
    let other = super::stored(&pool, ana, json!({})).await;
    let misplaced = json!({ "project": other, "text": "x", "path": "assets/brief.md" });
    let (said, refused) = call(address, &token, "script_write", misplaced).await;
    assert!(refused && said.contains("cannot hold"), "{said}");
    assert_eq!(document(&pool, ana, other).await.document.script, None);
}

#[sqlx::test]
async fn recipes_and_midi_are_offered(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (_, token) = member(&pool, "ana@example.com").await;
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
    let listed = super::post(address, &token, &body).await.json();
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("a list")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    for served in [
        "synth_new",
        "synth_bake",
        "script_read",
        "script_write",
        "synth_import",
        "synth_export",
    ] {
        assert!(names.contains(&served), "{served}: {names:?}");
    }
}
