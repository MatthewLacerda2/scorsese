//! `library` and `import`: the hosted server's files come from the user's
//! library, by id, and never from anybody else's.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, document, member};

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[sqlx::test]
async fn a_library_file_comes_into_a_project_once(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let item = common::hold(&pool, ana, SHA).await;
    let id = super::stored(&pool, ana, json!({})).await;

    let (listed, _) = call(address, &token, "library", json!({ "kind": "video" })).await;
    assert!(
        listed.starts_with(&format!("{item} — {SHA} (video")),
        "{listed}"
    );
    let (none, _) = call(address, &token, "library", json!({ "kind": "audio" })).await;
    assert!(none.starts_with("No files match"), "{none}");

    let bring = json!({ "project": id, "items": [item] });
    let (said, refused) = call(address, &token, "import", bring.clone()).await;
    assert!(!refused && said.contains("(added)"), "{said}");
    let stored = document(&pool, ana, id).await.document;
    assert_eq!(stored.assets.len(), 1);
    let asset = &stored.assets[0];
    assert_eq!(asset.sha256.as_deref(), Some(SHA));
    assert_eq!(
        asset.path.as_ref().map(|path| path.as_str().to_owned()),
        Some(format!("assets/{SHA}.mp4"))
    );

    let (again, _) = call(address, &token, "import", bring).await;
    assert!(again.contains("already in the project"), "{again}");
    assert_eq!(document(&pool, ana, id).await.document.assets.len(), 1);

    let (used, _) = call(address, &token, "library", json!({ "project": id })).await;
    assert!(used.starts_with(&item.to_string()), "{used}");
}

#[sqlx::test]
async fn another_users_file_is_not_in_your_library(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let (bob, token) = member(&pool, "bob@example.com").await;
    let hers = common::hold(&pool, ana, SHA).await;
    let his = super::stored(&pool, bob, json!({})).await;
    let (said, refused) = call(
        address,
        &token,
        "import",
        json!({ "project": his, "items": [hers] }),
    )
    .await;
    assert!(refused && said.contains("no file"), "{said}");
    assert!(document(&pool, bob, his).await.document.assets.is_empty());
    let (listed, _) = call(address, &token, "library", json!({})).await;
    assert!(listed.starts_with("No files match"), "{listed}");
}

#[sqlx::test]
async fn a_project_made_over_http_brings_its_library_files_in(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let item = common::hold(&pool, ana, SHA).await;
    let body = json!({ "name": "Reel", "platform": "instagram_reels", "assets": [item] });
    let made = common::request(address, "POST", "/api/projects", &[&token], Some(&body)).await;
    assert_eq!(made.status, 201, "{}", made.body);
    let made = made.json();
    assert_eq!(made["document"]["assets"][0]["sha256"], SHA, "{made}");
    assert_eq!(made["platform"], "instagram_reels");
}

#[sqlx::test]
async fn project_new_starts_a_project_for_a_platform_and_a_style(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (_, token) = member(&pool, "ana@example.com").await;
    let args = json!({ "name": "Ad", "platform": "tiktok_ad", "style": "flash_offer" });
    let (made, refused) = call(address, &token, "project_new", args).await;
    assert!(!refused && made.contains("script.md: the brief"), "{made}");
    let args = json!({ "name": "Ad", "platform": "youtube", "style": "flash_offer" });
    let (said, refused) = call(address, &token, "project_new", args).await;
    assert!(refused && said.contains("not made for"), "{said}");
}
