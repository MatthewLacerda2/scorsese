//! Renaming, describing and deleting a file — and the one refusal that
//! matters: a file a project uses stays.

use scorsese_core::hash_bytes;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use crate::common::{self, request};
use crate::{member, upload, video};

#[sqlx::test]
async fn a_file_is_renamed_and_described_for_the_assistant_to_read(pool: PgPool) {
    let bytes = video(&common::scratch("manage-rename-media"));
    let (address, _) = common::serve_with(pool.clone(), common::files("manage-rename")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "clip.mp4", &bytes).await;
    let path = format!("/api/library/{id}");
    let change = |body: Value| {
        let (ana, path) = (ana.clone(), path.clone());
        async move { request(address, "PATCH", &path, &[&ana], Some(&body)).await }
    };

    let described = change(json!({ "name": " Intro ", "description": "logo sting" })).await;
    assert_eq!(described.status, 200, "{}", described.body);
    assert_eq!(described.json()["name"], "Intro");
    assert_eq!(described.json()["description"], "logo sting");
    assert_eq!(change(json!({ "name": "  " })).await.status, 400);
    let cleared = change(json!({ "description": "" })).await.json();
    assert_eq!(
        (cleared["name"].clone(), cleared["description"].clone()),
        (json!("Intro"), Value::Null)
    );

    let search = request(
        address,
        "GET",
        "/api/library?search=INT&kind=video",
        &[&ana],
        None,
    )
    .await;
    assert_eq!(search.json()[0]["id"], id);
    let other = request(address, "GET", "/api/library?kind=audio", &[&ana], None).await;
    assert_eq!(other.json(), json!([]));
}

#[sqlx::test]
async fn a_file_a_project_uses_cannot_be_deleted_and_says_which(pool: PgPool) {
    let files = common::files("manage-delete");
    let storage = files.storage.clone();
    let bytes = video(&common::scratch("manage-delete-media"));
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (user, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "clip.mp4", &bytes).await;
    let sha256 = hash_bytes(&bytes);
    let item = format!("/api/library/{id}");

    let named = json!({ "name": "teaser" });
    let made = request(address, "POST", "/api/projects", &[&ana], Some(&named)).await;
    let project = format!("/api/projects/{}", made.json()["id"]);
    let mut document = request(address, "GET", &project, &[&ana], None)
        .await
        .json()["document"]
        .clone();
    let asset = |id: &str, hash: &str| json!({ "id": id, "kind": "video", "path": format!("assets/{hash}.mp4"), "sha256": hash });
    // A document may not name a file its owner does not have.
    document["assets"] = json!([asset("ghost", &"f".repeat(64))]);
    let save =
        |revision: i64, document: &Value| json!({ "revision": revision, "document": document });
    let refused = request(address, "PUT", &project, &[&ana], Some(&save(1, &document))).await;
    assert_eq!(refused.status, 400, "{}", refused.body);
    assert!(refused.body.contains("ghost"), "{}", refused.body);

    document["assets"] = json!([asset("intro", &sha256)]);
    let saved = request(address, "PUT", &project, &[&ana], Some(&save(1, &document))).await;
    assert_eq!(saved.status, 200, "{}", saved.body);
    let details = request(address, "GET", &item, &[&ana], None).await.json();
    assert_eq!(details["used_by"][0]["name"], "teaser");
    assert!(details["last_used_at"].is_number(), "{details}");

    let kept = request(address, "DELETE", &item, &[&ana], None).await;
    assert_eq!(kept.status, 409, "{}", kept.body);
    assert_eq!(kept.json()["projects"][0]["name"], "teaser");
    assert!(
        kept.json()["error"]
            .as_str()
            .unwrap()
            .contains("\u{201c}teaser\u{201d}")
    );

    document["assets"] = json!([]);
    let emptied = request(address, "PUT", &project, &[&ana], Some(&save(2, &document))).await;
    assert_eq!(emptied.status, 200, "{}", emptied.body);
    assert_eq!(
        request(address, "DELETE", &item, &[&ana], None)
            .await
            .status,
        204
    );
    assert!(
        !storage.library_file(user, &sha256, "mp4").exists(),
        "the file went too"
    );
    assert_eq!(
        request(address, "GET", &item, &[&ana], None).await.status,
        404
    );
}
