//! Pages on the web (#777): written by one call, kept with the project, and
//! there for the next — the way a recipe is.

use scorsese_core::AssetKind;
use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, document, member};

#[sqlx::test]
async fn a_page_is_written_kept_and_read_back(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, json!({})).await;

    let html = "<!doctype html><h1>Hello</h1>";
    let written = json!({ "project": id, "page": "title", "html": html });
    let (said, refused) = call(address, &token, "page_write", written).await;
    assert!(!refused && said.contains("pages/title.html"), "{said}");
    let (stored, kept) = projects::open_with_files(&pool, ana, id).await.unwrap();
    assert_eq!(stored.document.assets[0].kind, AssetKind::Html);
    assert_eq!(kept.get("pages/title.html"), Some(html));

    // A rewrite is a file change alone, and it is kept too.
    let again = json!({ "project": id, "page": "title", "html": "<p>Bye</p>" });
    let (said, refused) = call(address, &token, "page_write", again).await;
    assert!(!refused, "{said}");
    let asked = json!({ "project": id, "page": "title" });
    let (read, refused) = call(address, &token, "page_read", asked).await;
    assert!(!refused && read == "<p>Bye</p>", "{read}");
    assert_eq!(document(&pool, ana, id).await.document.assets.len(), 1);
}

#[sqlx::test]
async fn the_page_tools_are_offered(pool: PgPool) {
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
    for served in ["page_write", "page_read"] {
        assert!(names.contains(&served), "{served}: {names:?}");
    }
}
