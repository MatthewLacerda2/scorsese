//! How much one member may ask of the render queue (#1648): at most ten
//! renders and previews waiting at once, and at most so many asks a minute.
//! Previews of the project being edited, at the shape on screen, are not
//! counted: each new revision's makes the last one worthless.

use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, card, common, member, stored};

#[sqlx::test]
async fn ten_renders_wait_and_an_eleventh_is_refused(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let path = format!("/api/projects/{id}/renders");
    // Each loudness is another render.
    for lufs in 10..20 {
        let ask = json!({ "loudness": -lufs });
        let (status, body) = call(address, &cookie, "POST", &path, Some(ask)).await;
        assert_eq!(status, 202, "render {lufs}: {body}");
    }
    let (status, body) = call(
        address,
        &cookie,
        "POST",
        &path,
        Some(json!({ "loudness": -20 })),
    )
    .await;
    assert_eq!(status, 429, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("10 renders"),
        "{body}"
    );
    // One already waiting is still answered with its job.
    let (status, body) = call(
        address,
        &cookie,
        "POST",
        &path,
        Some(json!({ "loudness": -10 })),
    )
    .await;
    assert_eq!(status, 202, "{body}");
    let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(waiting, 10);

    // Somebody else's queue is their own.
    let (bea, theirs) = member(&pool, "bea@example.com").await;
    let id = stored(&pool, bea, &card(|_| {})).await;
    let path = format!("/api/projects/{id}/renders");
    let (status, body) = call(address, &theirs, "POST", &path, Some(json!({}))).await;
    assert_eq!(status, 202, "{body}");
}

#[sqlx::test]
async fn the_previews_of_an_edit_never_crowd_out_the_next_one(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let renders = format!("/api/projects/{id}/renders");
    for lufs in 10..19 {
        let ask = Some(json!({ "loudness": -lufs }));
        let (status, body) = call(address, &cookie, "POST", &renders, ask).await;
        assert_eq!(status, 202, "{body}");
    }
    // The tenth waiting job: a preview of the cut as it is.
    let previews = format!("/api/projects/{id}/previews");
    let shape = json!({ "resolution": "64x64", "quality": "half" });
    let (status, body) = call(address, &cookie, "POST", &previews, Some(shape.clone())).await;
    assert_eq!(status, 202, "{body}");

    // An edit, and the preview of it: the one before retires when its turn
    // comes, so it does not count against this one.
    for color in ["#112233", "#445566"] {
        let open = projects::open(&pool, ana, id).await.unwrap();
        let edited = card(|document| document["assets"][0]["color"] = json!(color));
        projects::save(&pool, ana, id, open.summary.revision, &edited)
            .await
            .unwrap();
        let (status, body) = call(address, &cookie, "POST", &previews, Some(shape.clone())).await;
        assert_eq!(status, 202, "{color}: {body}");
    }

    // A preview at another shape is another job, and the queue is full.
    let other = json!({ "resolution": "32x32", "quality": "half" });
    let (status, body) = call(address, &cookie, "POST", &previews, Some(other)).await;
    assert_eq!(status, 429, "{body}");
}

#[sqlx::test]
async fn asking_is_limited_per_minute(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let renders = format!("/api/projects/{id}/renders");
    let previews = format!("/api/projects/{id}/previews");
    let shape = json!({ "resolution": "64x64", "quality": "half" });
    // The same render asked again and again, as the editor's poll asks:
    // nothing new is queued, but each ask is counted, on both routes alike.
    for n in 0..120 {
        let (path, body) = if n % 2 == 0 {
            (&renders, json!({}))
        } else {
            (&previews, shape.clone())
        };
        let (status, said) = call(address, &cookie, "POST", path, Some(body)).await;
        assert_eq!(status, 202, "ask {n}: {said}");
    }
    let (status, body) = call(address, &cookie, "POST", &previews, Some(shape)).await;
    assert_eq!(status, 429, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("this minute"),
        "{body}"
    );
}
