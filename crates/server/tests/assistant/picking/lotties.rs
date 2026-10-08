//! A Lottie is offered beside footage and photos, shown by its GIF, and a
//! pick of one is imported exactly as `stock_import` imports one (#908): its
//! JSON kept under `pages/`, never an asset, and the model told the file a
//! page loads and the page that plays it.

use super::*;
use fake::LOTTIES;

#[sqlx::test]
async fn a_picked_lottie_lands_under_pages_with_the_page_that_plays_it(pool: PgPool) {
    let (script, fake) = (Script::new(Vec::new()), Fake::new());
    let (address, _) = stocked(&pool, &script, &fake).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let search = |kind| json!({ "project": id, "query": "wave", "kind": kind });
    let candidates: Vec<Value> = LOTTIES
        .iter()
        .map(|id| json!({ "kind": "lottie", "id": id }))
        .chain([json!({ "kind": "image", "id": 1 })])
        .collect();
    let asked = json!({ "question": "Which wave?", "candidates": candidates });
    script.replace(vec![
        calls("stock_search", search("lottie")),
        calls("stock_search", search("image")),
        calls("pick_stock", asked),
        answers("Waved."),
    ]);
    let sent = send(address, &who, id, "a waving mascot").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().expect("the turn has an id");
    let paused = finished(address, &who, turn).await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");
    let shown = &paused["turn"]["questions"][0]["candidates"];
    let lottie = &shown[1];
    assert_eq!(lottie["key"], "lottiefiles-lottie-8", "{shown}");
    assert_eq!(lottie["source"], "lottiefiles");
    assert_eq!(lottie["kind"], "lottie");
    assert_eq!(lottie["preview_url"], "https://cdn.example.invalid/8.gif");
    assert_eq!(lottie["look_url"], "https://cdn.example.invalid/8.gif");
    assert_eq!(
        shown[2]["key"], "pixabay-image-1",
        "one picker, mixed kinds"
    );

    let picked = json!({ "picked": ["lottiefiles-lottie-8"] });
    let resumed = answer(address, &who, turn, &picked).await;
    assert_eq!(resumed.status, 202, "{}", resumed.body);
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    assert_eq!(
        fake.imported(),
        ["https://cdn.example.invalid/8.json"],
        "only what was picked is downloaded"
    );

    let told = told(&script, 3);
    assert!(told.contains("The person picked lottie 8"), "{told}");
    assert!(told.contains("pages/lottie-8.json"), "{told}");
    assert!(
        told.contains("fetch(\"lottie-8.json\")"),
        "the page: {told}"
    );
    let imported = done["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["tool"] == "stock_import")
        .expect("the pick's import is recorded");
    assert_eq!(imported["client"], "user", "the person's own call");

    let kept: Vec<String> =
        sqlx::query_scalar("SELECT path FROM project_files WHERE project_id = $1")
            .bind(id)
            .fetch_all(&pool)
            .await
            .expect("the files are read");
    assert_eq!(kept, ["pages/lottie-8.json"]);
    let path = format!("/api/projects/{id}");
    let document = common::request(address, "GET", &path, &[&who], None).await;
    let assets = &document.json()["document"]["assets"];
    assert_eq!(assets, &json!([]), "a Lottie is never an asset");
}
