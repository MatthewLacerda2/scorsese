//! Previews (#542): the cut at a preview quality, reading the proxies a
//! finished render never reads, superseded by the next edit.
//!
//! The library video is red and its "proxy" a blue stand-in, so which one a
//! render decoded is the colour of its frames.

use std::path::Path;

use scorsese_core::{Project, hash_bytes};
use scorsese_render::{Resolution, frames};
use scorsese_server::Files;
use scorsese_server::db::UserId;
use scorsese_server::events::Events;
use scorsese_server::jobs::{Queue, Registry, State, kinds, work};
use scorsese_server::renders::job;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::job::finished;
use super::{call, card, common, member, stored};

/// A worker running renders and previews on `files`.
pub(super) async fn worker(pool: &PgPool, files: &Files) -> watch::Sender<bool> {
    let members = scorsese_server::db::member_pool(pool)
        .await
        .expect("the member pool connects");
    let handler = || {
        job::handler(
            files.renders.clone(),
            files.tools.clone(),
            files.storage.clone(),
        )
    };
    let registry = Registry::new()
        .register(kinds::RENDER, handler())
        .register(kinds::PREVIEW, handler());
    let (stop, stopping) = watch::channel(false);
    tokio::spawn(work(members, registry, Queue::new(Events::new()), stopping));
    stop
}

/// A red 32×32 video in `user`'s library, with a blue stand-in filed as its
/// proxy; a project showing it for six frames.
async fn red_in_library(pool: &PgPool, files: &Files, user: UserId) -> Project {
    let setup = "the test setup works";
    let make = |colour: &str, file: &Path| {
        std::fs::create_dir_all(file.parent().expect(setup)).expect(setup);
        let status = files
            .tools
            .ffmpeg()
            .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg(format!("color=c={colour}:s=32x32:d=1:r=30"))
            .args(["-pix_fmt", "yuv420p"])
            .arg(file)
            .status()
            .expect(setup);
        assert!(status.success(), "ffmpeg made the {colour} fixture");
    };
    let scratch = common::scratch("red").join("red.mp4");
    make("red", &scratch);
    let sha = hash_bytes(&std::fs::read(&scratch).expect(setup));
    let home = files.storage.library_file(user, &sha, "mp4");
    std::fs::create_dir_all(home.parent().expect(setup)).expect(setup);
    std::fs::rename(&scratch, home).expect(setup);
    make("blue", &files.storage.proxy(user, &sha));
    let media = json!({ "width": 32, "height": 32, "duration_seconds": 1.0, "has_alpha": false });
    sqlx::query(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, 'red', 'video', 'mp4', 1, $3)",
    )
    .bind(user.get())
    .bind(&sha)
    .bind(&media)
    .execute(pool)
    .await
    .expect(setup);
    card(|document| {
        document["assets"] = json!([{ "id": "shot", "kind": "video",
            "path": format!("assets/{sha}.mp4"), "sha256": sha, "media": media }]);
        document["tracks"][0]["clips"][0]["asset"] = json!("shot");
    })
}

/// The colour of frame 2 of render `id`'s file: `(red, blue)` channel means.
async fn colour(pool: &PgPool, files: &Files, id: &Value) -> (u64, u64) {
    let path: String = sqlx::query_scalar("SELECT path FROM renders WHERE id = $1")
        .bind(id.as_i64().expect("a render id"))
        .fetch_one(pool)
        .await
        .expect("the render is recorded");
    let file = files.renders.absolute(Path::new(&path));
    let raster = Resolution::new(32, 32).expect("a legal raster");
    let frame = frames::extract(&files.tools, &file, 2, raster).expect("the frame decodes");
    let mean = |at: usize| {
        let channel = frame.bytes().iter().skip(at).step_by(4);
        channel.map(|&v| u64::from(v)).sum::<u64>() / (32 * 32)
    };
    (mean(0), mean(2))
}

#[sqlx::test]
async fn a_preview_reads_the_proxy_and_the_finished_render_reads_the_original(pool: PgPool) {
    let files = common::files("preview-proxy");
    let _worker = worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &red_in_library(&pool, &files, ana).await).await;
    let mut made = Vec::new();
    for (path, ask) in [
        ("renders", json!({ "resolution": "32x32" })),
        (
            "previews",
            json!({ "resolution": "64x64", "quality": "half" }),
        ),
    ] {
        let (status, asked) = call(
            address,
            &cookie,
            "POST",
            &format!("/api/projects/{id}/{path}"),
            Some(ask),
        )
        .await;
        assert_eq!(status, 202, "{asked}");
        assert_eq!(
            asked["job"]["kind"],
            if path == "renders" {
                "render"
            } else {
                "preview"
            }
        );
        let job = finished(&pool, ana, asked["job"]["id"].as_i64().unwrap()).await;
        assert_eq!(job.state, State::Done, "{:?}", job.error);
        made.push(colour(&pool, &files, &job.result.unwrap()["render"]).await);
    }
    let [(red, blue), (proxy_red, proxy_blue)] = made[..] else {
        unreachable!()
    };
    assert!(
        red > 180 && blue < 80,
        "the finished render reads the original: {red} {blue}"
    );
    assert!(
        proxy_blue > 180 && proxy_red < 80,
        "the preview reads the proxy"
    );

    let (_, listed) = call(
        address,
        &cookie,
        "GET",
        &format!("/api/projects/{id}/renders"),
        None,
    )
    .await;
    assert_eq!(
        listed.as_array().unwrap().len(),
        1,
        "a preview is not a download: {listed}"
    );
}
