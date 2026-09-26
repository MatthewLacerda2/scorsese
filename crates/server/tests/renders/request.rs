//! Asking for a render over HTTP: the cache answers at once, the queue
//! otherwise, and another user reaches neither.

use scorsese_server::renders::{Settings, key};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, card, common, kept, member, stored};

#[sqlx::test]
async fn a_shape_the_output_formats_page_does_not_allow_is_refused_first(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let path = format!("/api/projects/{id}/renders");
    for (ask, says) in [
        (json!({ "container": "wmv", "video_codec": "h264" }), "wmv"),
        (
            json!({ "container": "mp3", "resolution": "64x36" }),
            "resolution",
        ),
        (json!({ "container": "webm" }), "webm"),
        (json!({ "resolution": "wide" }), "resolution"),
    ] {
        let (status, body) = call(address, &cookie, "POST", &path, Some(ask.clone())).await;
        assert_eq!(status, 400, "{ask}: {body}");
        assert!(
            body["error"].as_str().unwrap().contains(says),
            "{ask}: {body}"
        );
    }
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs, 0, "nothing was queued for a refused shape");
}

#[sqlx::test]
async fn a_new_render_is_queued_once_however_often_it_is_asked_for(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let path = format!("/api/projects/{id}/renders");

    let (status, first) = call(address, &cookie, "POST", &path, Some(json!({}))).await;
    assert_eq!(status, 202, "{first}");
    assert_eq!(first["render"], json!(null));
    assert_eq!(first["job"]["kind"], "render");
    assert_eq!(first["job"]["state"], "waiting");

    // `{}` and the defaults spelled out are one render.
    let spelled = json!({ "container": "mp4", "resolution": "1920x1080" });
    let (status, again) = call(address, &cookie, "POST", &path, Some(spelled)).await;
    assert_eq!(
        (status, &again["job"]["id"]),
        (202, &first["job"]["id"]),
        "{again}"
    );

    // Another shape is another render.
    let (_, other) = call(
        address,
        &cookie,
        "POST",
        &path,
        Some(json!({ "container": "mkv" })),
    )
    .await;
    assert_ne!(other["job"]["id"], first["job"]["id"], "{other}");
}

#[sqlx::test]
async fn a_kept_render_is_answered_at_once_and_one_whose_file_went_is_made_again(pool: PgPool) {
    let files = common::files("request-kept");
    let cache = files.renders.clone();
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let project = card(|_| {});
    let id = stored(&pool, ana, &project).await;
    let settings = Settings::from_ask(&Default::default()).unwrap();
    let render = kept(&pool, &cache, ana, id, &key(&project, &settings).unwrap()).await;
    super::idle_for(&pool, render.id, 10).await;

    let path = format!("/api/projects/{id}/renders");
    let (status, body) = call(address, &cookie, "POST", &path, Some(json!({}))).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["render"]["id"], render.id);
    assert_eq!(
        body["render"]["file"],
        format!("/api/renders/{}/file", render.id)
    );
    // Ten hours idle before asking; asking stamped it now.
    assert!(
        body["render"]["last_used_at"].as_i64() >= Some(render.last_used_at),
        "asking is use"
    );

    let (status, listed) = call(address, &cookie, "GET", &path, None).await;
    assert_eq!((status, listed[0]["id"].clone()), (200, json!(render.id)));

    // Cleared by hand: the row no longer promises a file.
    std::fs::remove_dir_all(cache.absolute(std::path::Path::new("users"))).unwrap();
    let (status, body) = call(address, &cookie, "POST", &path, Some(json!({}))).await;
    assert_eq!(status, 202, "{body}");
    assert_eq!(super::rows(&pool).await, 0);
}

#[sqlx::test]
async fn another_user_can_neither_ask_for_nor_download_a_render(pool: PgPool) {
    let files = common::files("request-isolation");
    let cache = files.renders.clone();
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let (_, bea) = member(&pool, "bea@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let render = kept(&pool, &cache, ana, id, &"a".repeat(64)).await;

    let (status, _) = call(
        address,
        &bea,
        "POST",
        &format!("/api/projects/{id}/renders"),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, 404);
    let (status, listed) = call(
        address,
        &bea,
        "GET",
        &format!("/api/projects/{id}/renders"),
        None,
    )
    .await;
    assert_eq!((status, listed), (200, json!([])));
    let (status, _) = call(
        address,
        &bea,
        "GET",
        &format!("/api/renders/{}/file", render.id),
        None,
    )
    .await;
    assert_eq!(status, 404);
}
