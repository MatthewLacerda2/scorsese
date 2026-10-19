//! A render of a project that shows a page: laid out in the capture spool
//! rather than the job's scratch folder, and written to that folder all the
//! same (Jujutsu Office, 2026-10-19, where every such render and preview
//! failed writing its mix into a folder nobody had made).

use scorsese_server::jobs::State;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, card, common, member, stored};

#[sqlx::test]
async fn a_project_showing_a_page_renders(pool: PgPool) {
    let files = common::files("job-page");
    files
        .renders
        .prepare()
        .expect("the cache and its spool are made");
    common::capturing::answering(&files.renders.captures());
    let _worker = super::job::worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let project = card(|document| {
        let assets = document["assets"].as_array_mut().unwrap();
        assets.push(json!({ "id": "title", "kind": "html", "path": "pages/title.html" }));
        let tracks = document["tracks"].as_array_mut().unwrap();
        tracks.push(json!({ "id": "v2", "kind": "video",
            "clips": [{ "id": "t", "asset": "title", "start": 0, "duration": 6 }] }));
    });
    let id = stored(&pool, ana, &project).await;
    sqlx::query("INSERT INTO project_files (project_id, user_id, path, content) VALUES ($1, $2, 'pages/title.html', $3)")
        .bind(id)
        .bind(ana.get())
        .bind("<body style='margin:0;background:#0f0'></body>")
        .execute(&pool)
        .await
        .expect("the test setup works");

    let path = format!("/api/projects/{id}/renders");
    let (status, asked) = call(
        address,
        &cookie,
        "POST",
        &path,
        Some(json!({ "resolution": "64x36" })),
    )
    .await;
    assert_eq!(status, 202, "{asked}");
    let job = super::job::finished(&pool, ana, asked["job"]["id"].as_i64().unwrap()).await;

    assert_eq!(job.state, State::Done, "{:?}", job.error);
    let said = job.result.unwrap().to_string();
    assert!(!said.contains("could not be captured"), "{said}");
}
