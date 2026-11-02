//! A member with no room left is refused a render or a preview at once
//! (#1647), with nothing queued; one the room cannot hold is not kept.

use scorsese_server::Files;
use scorsese_server::jobs::State;
use scorsese_server::renders::Quota;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::job::{finished, worker};
use super::{call, card, common, member, rows, stored};

#[sqlx::test]
async fn a_member_at_quota_asking_for_a_render_or_a_preview_is_refused(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    // Their library is full: one file the size of the quota.
    let quota = i64::try_from(scorsese_server::storage::DEFAULT_QUOTA.get()).unwrap();
    sqlx::query(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, 'huge.mp4', 'video', 'mp4', $3, '{}')",
    )
    .bind(ana.get())
    .bind("c".repeat(64))
    .bind(quota)
    .execute(&pool)
    .await
    .unwrap();

    for path in [
        format!("/api/projects/{id}/renders"),
        format!("/api/projects/{id}/previews"),
    ] {
        let (status, body) = call(address, &cookie, "POST", &path, Some(json!({}))).await;
        assert_eq!(status, 413, "{path}: {body}");
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("your storage is full"),
            "{path}: {body}"
        );
    }
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs, 0, "nothing was queued for a full account");
}

#[sqlx::test]
async fn a_render_larger_than_the_room_left_is_not_kept(pool: PgPool) {
    let files = common::files("full-kept");
    // Room for nothing an mp4 takes.
    let files = Files {
        storage: files.storage.with_quota(Quota::bytes(100)),
        ..files
    };
    let _worker = worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let path = format!("/api/projects/{id}/renders");

    let (status, asked) = call(
        address,
        &cookie,
        "POST",
        &path,
        Some(json!({ "resolution": "64x36" })),
    )
    .await;
    assert_eq!(status, 202, "nothing is kept yet: {asked}");
    let job = finished(&pool, ana, asked["job"]["id"].as_i64().unwrap()).await;

    assert_eq!(job.state, State::Failed);
    let error = job.error.unwrap_or_default();
    assert!(error.contains("your storage is full"), "{error}");
    assert_eq!(rows(&pool).await, 0, "no render was kept");
}
