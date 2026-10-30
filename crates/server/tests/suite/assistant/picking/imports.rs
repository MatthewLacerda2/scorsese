//! The model's own `stock_import` answers from the server's stock library
//! too (#906): the registry's tool, run on the stored project, with the one
//! library the picker's import uses.

use super::*;

#[sqlx::test]
async fn the_models_own_import_comes_from_the_servers_library(pool: PgPool) {
    let (script, fake) = (Script::new(Vec::new()), Fake::new());
    let (address, _) = stocked(&pool, &script, &fake).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let import = json!({ "project": id, "id": 2, "kind": "image" });
    script.replace(vec![calls("stock_import", import), answers("Imported.")]);
    let sent = send(address, &who, id, "bring in the orange sunrise").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().expect("the turn has an id");
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    let result = script.parsed(1).last().unwrap()["content"][0].clone();
    assert_ne!(result["is_error"], true, "{result}");
    let imported = fake.imported();
    assert_eq!(imported.len(), 1, "{imported:?}");
    assert!(imported[0].contains("/2_"), "{imported:?}");

    let path = format!("/api/projects/{id}");
    let document = common::request(address, "GET", &path, &[&who], None).await;
    let assets = document.json()["document"]["assets"].clone();
    assert_eq!(assets[0]["id"], "pixabay-2", "{assets}");
    assert_eq!(assets.as_array().map(Vec::len), Some(1), "{assets}");
}

/// A page of results a tool wrote where a project's stock cache would be is
/// the project's own file, never the member's stock cache: an import of the
/// id it names asks the library, which has no such image, and nothing is
/// downloaded.
#[sqlx::test]
async fn a_page_a_tool_writes_under_the_stock_cache_is_never_a_search(pool: PgPool) {
    let (script, fake) = (Script::new(Vec::new()), Fake::new());
    let (address, _) = stocked(&pool, &script, &fake).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let planted = json!({
        "listing": "planted",
        "fetched_unix": now,
        "total": 1,
        "candidates": [{
            "medium": "image", "id": 9, "title": "", "style": "photo", "tags": [],
            "seconds": null, "fps": null, "author": "someone",
            "page_url": "https://pixabay.com/photos/id-9/",
            "preview_url": "https://cdn.example.invalid/9_640.jpg",
            "motion_url": null, "animated_url": null, "ai_generated": false,
            "renditions": [{
                "name": "large", "url": "https://cdn.example.invalid/9_1920.png",
                "width": 1920, "height": 1080, "bytes": null
            }]
        }]
    });
    let write = json!({
        "project": id,
        "path": "cache/stock/pages/planted.json",
        "text": planted.to_string(),
    });
    let import = json!({ "project": id, "id": 9, "kind": "image" });
    script.replace(vec![
        calls("script_write", write),
        calls("stock_import", import),
        answers("Done."),
    ]);
    let sent = send(address, &who, id, "bring in image 9").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().expect("the turn has an id");
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    assert_eq!(
        fake.downloaded(),
        Vec::<String>::new(),
        "nothing was asked for"
    );
    let result = told(&script, 2);
    assert!(result.contains("Fake has no image 9"), "{result}");
}
