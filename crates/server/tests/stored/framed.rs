//! A stored project from before documents carried their shape (#1278): the
//! platform it was started for decides its frame, and without one its footage.

use scorsese_core::Ratio;
use scorsese_server::db;
use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::account;

/// A v51 document whose only picture is a landscape shot.
fn landscape_v51() -> String {
    json!({
        "schema_version": 51,
        "name": "Before shapes",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [{ "id": "shot", "kind": "generated_video", "prompt": "a field",
                     "state": "sketch", "video": { "aspect": "16:9" } }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c", "asset": "shot", "start": 0, "duration": 30 }
        ]}]
    })
    .to_string()
}

#[sqlx::test]
async fn the_platform_a_project_was_started_for_frames_it_on_start(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let mut ids = Vec::new();
    for platform in [Some("tiktok"), None] {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO projects (user_id, document, platform) \
             VALUES ($1, $2::jsonb, $3) RETURNING id",
        )
        .bind(ana.get())
        .bind(landscape_v51())
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
        ids.push(id);
    }

    assert_eq!(projects::migrate_stored(&pool).await.unwrap(), 2);
    let members = db::member_pool(&pool).await.unwrap();
    let tiktok = projects::open(&members, ana, ids[0]).await.unwrap();
    assert_eq!(tiktok.document.aspect, Ratio::TALL, "the platform's shape");
    let plain = projects::open(&members, ana, ids[1]).await.unwrap();
    assert_eq!(plain.document.aspect, Ratio::WIDE, "its footage's shape");
}
