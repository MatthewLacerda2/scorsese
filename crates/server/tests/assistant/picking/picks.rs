//! The pick resumes the same turn, with what was picked already imported.

use super::*;
use crate::script::CHARGED;

#[sqlx::test]
async fn a_picker_waits_free_and_imports_exactly_what_was_picked(pool: PgPool) {
    let (script, fake) = (script(&[1, 2, 3], "Opened on the sunrises."), Fake::new());
    let (address, who, id, turn, paused) = picking(&pool, &script, &fake).await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");
    let shown = &paused["turn"]["questions"][1];
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
    assert!(fake.downloaded().is_empty(), "showing downloads nothing");
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
        done["turn"]["questions"][1]["picked"],
        json!(["pixabay-image-1", "pixabay-image-3"]),
        "in the order shown"
    );
    let downloaded = fake.downloaded();
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
    let imported = &done["tools"][0];
    assert_eq!(imported["tool"], "stock_import", "{done}");
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

#[sqlx::test]
async fn none_picked_or_words_instead_resume_with_what_was_said(pool: PgPool) {
    let (script, fake) = (script(&[1, 2], "Searching for darker ones."), Fake::new());
    let (address, who, _, turn, paused) = picking(&pool, &script, &fake).await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");
    let stranger = json!({ "picked": ["pixabay-image-3"] });
    let refused = answer(address, &who, turn, &stranger).await;
    assert_eq!(
        refused.status, 400,
        "image 3 was not shown: {}",
        refused.body
    );
    let nothing = answer(address, &who, turn, &json!({})).await;
    assert_eq!(nothing.status, 400, "{}", nothing.body);

    let none = json!({ "picked": [], "answer": "something darker" });
    assert_eq!(answer(address, &who, turn, &none).await.status, 202);
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    assert_eq!(done["turn"]["questions"][1]["picked"], json!([]));
    assert_eq!(done["turn"]["questions"][1]["answer"], "something darker");
    let told = told(&script, 2);
    assert!(told.contains("picked none of these"), "{told}");
    assert!(told.contains("They wrote: something darker"), "{told}");
    assert!(fake.downloaded().is_empty(), "nothing was imported");
    assert!(done["tools"].as_array().unwrap().is_empty(), "{done}");
}

#[sqlx::test]
async fn a_typed_message_answers_a_picker(pool: PgPool) {
    let (script, fake) = (script(&[1, 2], "Looking for footage."), Fake::new());
    let (address, who, id, turn, _) = picking(&pool, &script, &fake).await;
    let typed = send(address, &who, id, "none, use footage instead").await;
    assert_eq!(typed.status, 202, "{}", typed.body);
    assert_eq!(
        typed.json()["id"],
        turn,
        "a message answers, it starts nothing"
    );
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["answer"], "Looking for footage.", "{done}");
    assert!(done["turn"]["questions"][1].get("picked").is_none());
    let told = told(&script, 2);
    assert!(told.contains("answered in words instead"), "{told}");
    assert!(told.contains("none, use footage instead"), "{told}");
}
