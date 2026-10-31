//! A stored project whose asset ids are no safe file names: renamed on start,
//! every reference with them, and stored valid.

use scorsese_server::db;
use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::account;

#[sqlx::test]
async fn a_stored_project_s_bad_ids_are_renamed_on_start(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let v55 = json!({
        "schema_version": 55,
        "name": "Ids with slashes",
        "timeline_fps": { "num": 30, "den": 1 },
        "aspect": "16:9",
        "assets": [
            { "id": "../../escaped", "kind": "color", "color": "#101820" },
            { "id": "escaped", "kind": "color", "color": "#203040" }
        ],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c", "asset": "../../escaped", "start": 0, "duration": 30 }
        ]}]
    });
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (user_id, document) VALUES ($1, $2::jsonb) RETURNING id",
    )
    .bind(ana.get())
    .bind(v55.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(projects::migrate_stored(&pool).await.unwrap(), 1);
    let members = db::member_pool(&pool).await.unwrap();
    let stored = projects::open(&members, ana, id).await.unwrap().document;
    stored.validate().expect("the stored document validates");
    let ids: Vec<&str> = stored.assets.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(
        ids,
        ["escaped-2", "escaped"],
        "the id that kept the rule kept it"
    );
    assert_eq!(stored.tracks[0].clips[0].asset.as_str(), "escaped-2");
}
