//! The pick resumes the same turn, with what was picked already imported.

use super::*;
use crate::script::CHARGED;

#[sqlx::test]
async fn a_picker_waits_free_and_imports_exactly_what_was_picked(pool: PgPool) {
    let Picking {
        script,
        fake,
        address,
        who,
        project: id,
        turn,
        paused,
    } = picking(&pool, &[1, 2, 3], "Opened on the sunrises.").await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");
    let searched = &paused["tools"][0];
    assert_eq!(searched["tool"], "stock_search", "{paused}");
    assert_eq!(searched["client"], "assistant", "the model's own search");
    let shown = &paused["turn"]["questions"][0];
    assert_eq!(shown["question"], "Which sunrise do you like?", "{shown}");
    let keys: Vec<&str> = shown["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| one["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        keys,
        ["pixabay-image-1", "pixabay-image-2", "pixabay-image-3"]
    );
    let first = &shown["candidates"][0];
    assert_eq!(first["source"], "pixabay");
    assert_eq!(
        first["preview_url"],
        "https://cdn.example.invalid/1_640.jpg"
    );
    assert!(shown["picked"].is_null(), "{shown}");
    assert!(fake.imported().is_empty(), "showing imports nothing");
    let previews = fake.downloaded();
    assert_eq!(previews.len(), 3, "the search's sheet: {previews:?}");
    assert_eq!(script.requests().len(), 2, "nothing runs while it waits");
    assert_eq!(paused["turn"]["charged_micros"], 2 * CHARGED);
    let names: Vec<String> = script.requests()[0]
        .tools
        .iter()
        .map(|tool| tool.name.clone())
        .collect();
    assert_eq!(names[names.len() - 2..], ["ask_user", "pick_stock"]);

    let picked = json!({ "picked": ["pixabay-image-3", "pixabay-image-1"] });
    let resumed = answer(address, &who, turn, &picked).await;
    assert_eq!(resumed.status, 202, "{}", resumed.body);
    assert_eq!(resumed.json()["id"], turn, "the same turn");
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    assert_eq!(
        done["turn"]["questions"][0]["picked"],
        json!(["pixabay-image-1", "pixabay-image-3"]),
        "in the order shown"
    );
    let downloaded = fake.imported();
    assert_eq!(downloaded.len(), 2, "{downloaded:?}");
    assert!(downloaded[0].contains("/1_") && downloaded[1].contains("/3_"));

    let told = told(&script, 2);
    assert!(
        told.contains("The person picked image 1, image 3"),
        "{told}"
    );
    assert!(
        told.contains("`pixabay-1`") && told.contains("`pixabay-3`"),
        "{told}"
    );
    let imported = done["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["tool"] == "stock_import")
        .expect("the pick's import is recorded");
    assert_eq!(
        imported["client"], "user",
        "the pick is the person's own call"
    );
    assert_eq!(
        imported["arguments"]["picked"],
        json!(["image 1", "image 3"])
    );

    let path = format!("/api/projects/{id}");
    let document = common::request(address, "GET", &path, &[&who], None).await;
    let assets = document.json()["document"]["assets"].clone();
    let ids: Vec<&str> = assets
        .as_array()
        .unwrap()
        .iter()
        .map(|one| one["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["pixabay-1", "pixabay-3"], "{assets}");
}
