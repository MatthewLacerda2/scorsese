//! How much one member may ask of the render queue (#1648): at most ten
//! renders and previews waiting at once, and at most so many queued a
//! minute. A new preview cancels the project's waiting ones at its shape, so
//! an edit's previews never fill the queue.

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
    let (status, first) = call(address, &cookie, "POST", &previews, Some(shape.clone())).await;
    assert_eq!(status, 202, "{first}");

    // An edit, and the preview of it: the one before is cancelled, so the
    // queue never fills with pictures of revisions gone by.
    let mut asked = vec![first["job"]["id"].as_i64().unwrap()];
    for color in ["#112233", "#445566"] {
        edit(&pool, ana, id, color).await;
        let (status, body) = call(address, &cookie, "POST", &previews, Some(shape.clone())).await;
        assert_eq!(status, 202, "{color}: {body}");
        asked.push(body["job"]["id"].as_i64().unwrap());
    }
    let states: Vec<String> =
        sqlx::query_scalar("SELECT state FROM jobs WHERE id = ANY($1) ORDER BY id")
            .bind(&asked)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(states, ["cancelled", "cancelled", "waiting"]);

    // A preview at another shape is another job, and the queue is full.
    let other = json!({ "resolution": "32x32", "quality": "half" });
    let (status, body) = call(address, &cookie, "POST", &previews, Some(other)).await;
    assert_eq!(status, 429, "{body}");
}

/// Save `user`'s project `id` with its card painted `color`.
async fn edit(pool: &PgPool, user: scorsese_server::db::UserId, id: i64, color: &str) {
    let open = projects::open(pool, user, id)
        .await
        .expect("the project opens");
    let edited = card(|document| document["assets"][0]["color"] = json!(color));
    projects::save(pool, user, id, open.summary.revision, &edited)
        .await
        .expect("the edit is saved");
}

#[sqlx::test]
async fn queueing_is_limited_per_minute_and_asking_again_never_is(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let renders = format!("/api/projects/{id}/renders");
    let previews = format!("/api/projects/{id}/previews");
    let shape = json!({ "resolution": "64x64", "quality": "half" });
    // The same render asked again and again, as the editor's poll asks:
    // nothing new is queued, and nothing is counted.
    for n in 0..130 {
        let (status, said) = call(address, &cookie, "POST", &renders, Some(json!({}))).await;
        assert_eq!(status, 202, "ask {n}: {said}");
    }
    // A new preview after each edit: each one queued is counted, both routes
    // together — the render above was the first.
    for n in 1..120 {
        edit(&pool, ana, id, &format!("#{n:06x}")).await;
        let (status, said) = call(address, &cookie, "POST", &previews, Some(shape.clone())).await;
        assert_eq!(status, 202, "preview {n}: {said}");
    }
    edit(&pool, ana, id, "#ffffff").await;
    let (status, body) = call(address, &cookie, "POST", &previews, Some(shape)).await;
    assert_eq!(status, 429, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("this minute"),
        "{body}"
    );
}
