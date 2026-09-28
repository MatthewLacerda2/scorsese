//! The files a template holds, the editor's way in, and the start's migration.

use serde_json::json;
use sqlx::postgres::PgPool;

use crate::common::{self, request};
use crate::{episode, member, only_template, save_intro};
use scorsese_server::templates;

/// A library file a template uses stays, as one a project uses does — and
/// deleting the template is what lets it go, leaving the videos alone.
#[sqlx::test]
async fn a_file_a_template_holds_cannot_be_deleted_until_the_template_is(pool: PgPool) {
    let (address, state) = common::serve_with(pool.clone(), common::files("templates-file")).await;
    let (ana, browser) = member(&pool, "ana@example.com").await;
    let project = episode(&pool, ana).await;
    save_intro(&state, ana, project, "Intro")
        .await
        .expect("saved");
    let template = only_template(&state, ana).await;
    let deleted = request(
        address,
        "DELETE",
        &format!("/api/projects/{project}"),
        &[&browser],
        None,
    )
    .await;
    assert_eq!(deleted.status, 204, "{}", deleted.body);

    let item: i64 = sqlx::query_scalar("SELECT id FROM library_items")
        .fetch_one(&pool)
        .await
        .unwrap();
    let path = format!("/api/library/{item}");
    let kept = request(address, "DELETE", &path, &[&browser], None).await;
    assert_eq!(kept.status, 409, "{}", kept.body);
    assert_eq!(kept.json()["templates"][0]["name"], "Intro");
    assert!(
        kept.body.contains("template \u{201c}Intro\u{201d}"),
        "{}",
        kept.body
    );
    let details = request(address, "GET", &path, &[&browser], None)
        .await
        .json();
    assert_eq!(details["templates"][0]["id"], template);

    let listed = request(address, "GET", "/api/templates", &[&browser], None).await;
    assert_eq!(listed.json()[0]["name"], "Intro");
    let gone = format!("/api/templates/{template}");
    assert_eq!(
        request(address, "DELETE", &gone, &[&browser], None)
            .await
            .status,
        204
    );
    assert_eq!(
        request(address, "DELETE", &gone, &[&browser], None)
            .await
            .status,
        404
    );
    assert_eq!(
        request(address, "DELETE", &path, &[&browser], None)
            .await
            .status,
        204
    );
}

/// The editor saves a selection and puts a template in at the playhead through
/// its own route, with no revision: neither is computed on what a drag saw.
#[sqlx::test]
async fn the_editor_saves_a_selection_and_inserts_at_the_playhead(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, browser) = member(&pool, "ana@example.com").await;
    let project = episode(&pool, ana).await;
    let tool = |name: &str, body: serde_json::Value| {
        let (path, browser) = (
            format!("/api/projects/{project}/tools/{name}"),
            browser.clone(),
        );
        async move { request(address, "POST", &path, &[&browser], Some(&body)).await }
    };

    let selection = json!({ "clips": ["c-title"], "name": "Lower third" });
    let saved = tool("template_save", json!({ "arguments": selection })).await;
    assert_eq!(saved.status, 200, "{}", saved.body);
    assert_eq!(
        saved.json()["project"]["revision"],
        1,
        "saving changes no project"
    );

    let template = request(address, "GET", "/api/templates", &[&browser], None)
        .await
        .json()[0]["id"]
        .clone();
    let at = json!({ "arguments": { "template": template, "at_seconds": 10 } });
    let inserted = tool("template_insert", at).await;
    assert_eq!(inserted.status, 200, "{}", inserted.body);
    let body = inserted.json();
    assert_eq!(body["project"]["revision"], 2);
    // One video lane in the template, so the project's first: free at 10s.
    let v1 = &body["project"]["document"]["tracks"][0]["clips"];
    assert_eq!(
        (v1[1]["id"].clone(), v1[1]["start"].clone()),
        (json!("c-title-2"), json!(300))
    );
}

#[sqlx::test]
async fn templates_already_current_are_not_touched_on_start(pool: PgPool) {
    let (_, state) = common::serve_with(pool.clone(), common::files("templates-start")).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    save_intro(&state, ana, episode(&pool, ana).await, "Intro")
        .await
        .expect("saved");
    assert_eq!(templates::migrate_stored(&pool).await.unwrap(), 0);
}
