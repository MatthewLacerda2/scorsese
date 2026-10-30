//! A stored document is validated when a render is asked for and again by the
//! job, whatever stored it.

use scorsese_server::db;
use scorsese_server::jobs::{State, kinds, store};
use scorsese_server::renders::job::Payload;
use scorsese_server::renders::{Settings, key};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::job::{finished, worker};
use super::{call, card, common, member, stored};

/// A stored document that never went through a save — or one stored before
/// saves were validated — is validated when it is asked for and again by the
/// job, before anything is laid out or rendered.
#[sqlx::test]
async fn a_document_whose_path_leaves_the_project_is_refused_by_the_ask_and_the_job(pool: PgPool) {
    let files = common::files("job-escape");
    let _worker = worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let project = card(|document| {
        let assets = document["assets"].as_array_mut().unwrap();
        assets.push(json!({ "id": "leak", "kind": "image", "path": "assets/../../outside.png" }));
    });
    let id = stored(&pool, ana, &project).await;

    let (status, refused) = call(
        address,
        &cookie,
        "POST",
        &format!("/api/projects/{id}/renders"),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, 400, "{refused}");
    assert!(
        refused.to_string().contains("escapes the project root"),
        "{refused}"
    );

    // The job, handed the same document directly.
    let settings = Settings::from_ask(&Default::default()).unwrap();
    let payload = Payload {
        project: id,
        key: key(&project, &settings).unwrap(),
        settings,
        document: serde_json::to_value(&project).unwrap(),
    };
    let members = db::member_pool(&pool).await.unwrap();
    let mut tx = db::scoped(&members, ana).await.unwrap();
    let payload = serde_json::to_value(payload).unwrap();
    let queued = store::enqueue(&mut tx, kinds::RENDER, &payload)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let job = finished(&pool, ana, queued.id).await;
    assert_eq!(job.state, State::Failed);
    let error = job.error.unwrap();
    for says in [
        "`leak`",
        "assets/../../outside.png",
        "escapes the project root",
    ] {
        assert!(error.contains(says), "{says}: {error}");
    }
    assert_eq!(super::rows(&pool).await, 0);
}
