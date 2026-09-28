//! Preview proxies (#542): queued with a heavy video when it arrives, made by
//! the worker into the cache, and removed with the item.

use std::time::{Duration, Instant};

use scorsese_server::jobs::{self, kinds};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use crate::common::{self, request};
use crate::{member, upload};

/// A 1280×720 video a few frames long — larger than a proxy on its short
/// side, so worth one — made by ffmpeg under `directory`.
fn wide(directory: &std::path::Path) -> Vec<u8> {
    let file = directory.join("wide.mp4");
    let output = common::tools()
        .ffmpeg()
        .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg("testsrc=s=1280x720:d=0.2:r=30")
        .args(["-pix_fmt", "yuv420p"])
        .arg(&file)
        .output()
        .expect("ffmpeg runs");
    assert!(output.status.success(), "{output:?}");
    std::fs::read(file).expect("the fixture reads")
}

#[sqlx::test]
async fn a_heavy_video_gets_a_proxy_on_arrival_and_loses_it_with_the_item(pool: PgPool) {
    let files = common::files("proxy-arrival");
    let storage = files.storage.clone();
    let registry = kinds::registry(&files);
    let bytes = wide(&common::scratch("proxy-arrival-media"));
    let (address, state) = common::serve_with(pool.clone(), files).await;
    let (_stop, stopping) = watch::channel(false);
    tokio::spawn(jobs::work(
        state.pool.clone(),
        registry,
        state.jobs.clone(),
        stopping,
    ));
    let (user, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "wide.mp4", &bytes).await;

    let proxy = storage.proxy(user, &scorsese_core::hash_bytes(&bytes));
    let patience = Instant::now() + Duration::from_secs(60);
    while !proxy.is_file() && Instant::now() < patience {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(proxy.is_file(), "the proxy is made at {}", proxy.display());
    let listed = jobs::store::list(&state.pool, user).await.unwrap();
    assert!(
        listed.iter().any(|job| job.kind == "proxy"),
        "queued as a proxy job: {listed:?}"
    );

    let deleted = request(
        address,
        "DELETE",
        &format!("/api/library/{id}"),
        &[&ana],
        None,
    )
    .await;
    assert_eq!(deleted.status, 204, "{}", deleted.body);
    assert!(!proxy.exists(), "the proxy goes with its item");
}

#[sqlx::test]
async fn a_small_video_gets_no_proxy(pool: PgPool) {
    let files = common::files("proxy-small");
    let bytes = crate::video(&common::scratch("proxy-small-media"));
    let (address, state) = common::serve_with(pool.clone(), files).await;
    let (user, ana) = member(&pool, "ana@example.com").await;
    upload(address, &ana, "small.mp4", &bytes).await;
    let listed = jobs::store::list(&state.pool, user).await.unwrap();
    let kinds: Vec<&str> = listed.iter().map(|job| job.kind.as_str()).collect();
    assert_eq!(kinds, ["thumbnail"], "64×64 needs no proxy");
}
