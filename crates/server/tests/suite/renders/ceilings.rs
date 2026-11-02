//! How big and how long a render may be on the shared machine (#1634): 4K,
//! 60 frames a second and an hour at most, refused before a job is queued.

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{call, card, common, member, stored};

/// An hour of the timeline at its own 30 frames a second.
const HOUR: u64 = 30 * 60 * 60;

/// A card `frames` long on a 30 fps timeline.
fn lasting(frames: u64) -> scorsese_core::Project {
    card(|document| document["tracks"][0]["clips"][0]["duration"] = json!(frames))
}

async fn jobs(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(pool)
        .await
        .expect("the jobs count")
}

#[sqlx::test]
async fn the_largest_render_the_machine_takes_is_queued(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &lasting(HOUR)).await;
    let path = format!("/api/projects/{id}/renders");
    for ask in [
        json!({ "resolution": "3840x2160", "fps": "60" }),
        json!({ "resolution": "2160x3840", "fps": "60" }),
        json!({ "resolution": "3840x2160", "range": "0:108000" }),
    ] {
        let (status, body) = call(address, &cookie, "POST", &path, Some(ask.clone())).await;
        assert_eq!(status, 202, "{ask}: {body}");
    }
}

#[sqlx::test]
async fn a_render_past_a_ceiling_is_refused_in_words_before_it_is_queued(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let hour = stored(&pool, ana, &lasting(HOUR)).await;
    let longer = stored(&pool, ana, &lasting(HOUR + 1)).await;
    let cases: [(i64, Value, &str); 6] = [
        (hour, json!({ "resolution": "3842x2160" }), "3840"),
        (hour, json!({ "resolution": "2160x3842" }), "3840"),
        (hour, json!({ "resolution": "3000x2162" }), "2160"),
        (hour, json!({ "fps": "61" }), "60 frames a second"),
        (longer, json!({}), "60 minutes"),
        (longer, json!({ "range": "0:108001" }), "60 minutes"),
    ];
    for (id, ask, says) in cases {
        let path = format!("/api/projects/{id}/renders");
        let (status, body) = call(address, &cookie, "POST", &path, Some(ask.clone())).await;
        assert_eq!(status, 400, "{ask}: {body}");
        let said = body["error"].as_str().unwrap_or_default();
        assert!(said.contains(says), "{ask}: {said}");
    }
    // A range inside the hour of a longer timeline is fine.
    let path = format!("/api/projects/{longer}/renders");
    let inside = Some(json!({ "range": "1:108001" }));
    let (status, body) = call(address, &cookie, "POST", &path, inside).await;
    assert_eq!(status, 202, "{body}");
    assert_eq!(
        jobs(&pool).await,
        1,
        "nothing was queued for a refused render"
    );
}

#[sqlx::test]
async fn a_preview_takes_the_same_ceilings(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let longer = stored(&pool, ana, &lasting(HOUR + 1)).await;
    let path = format!("/api/projects/{longer}/previews");
    let (status, body) = call(address, &cookie, "POST", &path, Some(json!({}))).await;
    assert_eq!(status, 400, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("60 minutes"),
        "{body}"
    );
    let huge = Some(json!({ "resolution": "8000x4500", "quality": "full" }));
    let (status, body) = call(address, &cookie, "POST", &path, huge).await;
    assert_eq!(status, 400, "{body}");
    assert_eq!(jobs(&pool).await, 0);
}

#[sqlx::test]
async fn a_fast_timeline_is_said_and_its_sound_alone_is_rendered(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let fast = card(|document| document["timeline_fps"] = json!({ "num": 120, "den": 1 }));
    let id = stored(&pool, ana, &fast).await;
    let path = format!("/api/projects/{id}/renders");
    let (status, body) = call(address, &cookie, "POST", &path, Some(json!({}))).await;
    assert_eq!(status, 400, "{body}");
    let said = body["error"].as_str().unwrap_or_default();
    assert!(said.contains("timeline runs at 120"), "{said}");
    for ask in [json!({ "fps": "60" }), json!({ "container": "mp3" })] {
        let (status, body) = call(address, &cookie, "POST", &path, Some(ask.clone())).await;
        assert_eq!(status, 202, "{ask}: {body}");
    }
}
