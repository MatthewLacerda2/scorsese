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
