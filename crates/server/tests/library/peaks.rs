//! A sound's peaks (#1572): read by the worker beside its waveform picture,
//! listed on its tile, served once with long-lived caching, and nobody else's.

use std::path::Path;
use std::time::{Duration, Instant};

use scorsese_render::testing::generate;
use scorsese_server::jobs::{self, kinds};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use crate::common::{self, request};
use crate::{member, upload, video};

/// Two seconds of lavfi's `sine`, at its eighth of full scale.
fn sound(directory: &Path) -> Vec<u8> {
    let file = directory.join("line.wav");
    generate(
        &common::tools(),
        &file,
        &["-f", "lavfi", "-i", "sine=duration=2"],
    );
    std::fs::read(file).expect("the fixture reads")
}

#[sqlx::test]
async fn the_worker_reads_a_sounds_peaks_and_they_are_served_to_keep(pool: PgPool) {
    let files = common::files("peaks-drawn");
    let registry = kinds::registry(&files, Default::default());
    let bytes = sound(&common::scratch::dir("peaks-drawn-media"));
    let (address, state) = common::serve_with(pool.clone(), files).await;
    let (_stop, stopping) = watch::channel(false);
    let worker = tokio::spawn(jobs::work(
        state.pool.clone(),
        registry,
        state.jobs.clone(),
        stopping,
    ));
    common::release_at_teardown(&pool, &state.pool, [worker.abort_handle()]);
    let (_, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "line.wav", &bytes).await;

    let listed = request(address, "GET", "/api/library", &[&ana], None).await;
    let path = format!("/api/library/{id}/peaks");
    assert_eq!(listed.json()[0]["peaks"], path.as_str());
    let patience = Instant::now() + Duration::from_secs(30);
    let read = loop {
        let asked = request(address, "GET", &path, &[&ana], None).await;
        if asked.status == 200 || Instant::now() > patience {
            break asked;
        }
        assert_eq!(asked.json()["pending"], true, "{}", asked.body);
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    assert_eq!(read.status, 200, "{}", read.body);
    assert_eq!(
        read.header("content-type"),
        Some("application/octet-stream")
    );
    assert!(
        read.header("cache-control")
            .is_some_and(|kept| kept.contains("immutable")),
        "{:?}",
        read.header("cache-control")
    );
    assert_eq!(
        read.bytes.len(),
        4 + 80,
        "the rate, then two seconds at forty"
    );
    assert_eq!(read.bytes[..4], 40.0_f32.to_le_bytes());
    assert!(
        read.bytes[4..].iter().all(|level| level.abs_diff(32) <= 1),
        "an eighth of full scale: {:?}",
        &read.bytes[4..]
    );
}

#[sqlx::test]
async fn peaks_not_read_yet_are_pending_and_a_picture_has_none(pool: PgPool) {
    let scratch = common::scratch::dir("peaks-pending-media");
    let (address, _) = common::serve_with(pool.clone(), common::files("peaks-pending")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let line = upload(address, &ana, "line.wav", &sound(&scratch)).await;
    let clip = upload(address, &ana, "clip.mp4", &video(&scratch)).await;

    let pending = request(
        address,
        "GET",
        &format!("/api/library/{line}/peaks"),
        &[&ana],
        None,
    )
    .await;
    assert_eq!(
        (pending.status, &pending.json()["pending"]),
        (404, &true.into())
    );

    let none = request(
        address,
        "GET",
        &format!("/api/library/{clip}/peaks"),
        &[&ana],
        None,
    )
    .await;
    assert_eq!((none.status, &none.json()["pending"]), (404, &false.into()));
    let listed = request(address, "GET", "/api/library?kind=video", &[&ana], None).await;
    assert!(listed.json()[0].get("peaks").is_none(), "{}", listed.body);
}

#[sqlx::test]
async fn another_users_peaks_are_not_there_for_them(pool: PgPool) {
    let scratch = common::scratch::dir("peaks-iso-media");
    let (address, _) = common::serve_with(pool.clone(), common::files("peaks-iso")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (_, bia) = member(&pool, "bia@example.com").await;
    let line = upload(address, &ana, "line.wav", &sound(&scratch)).await;
    let path = format!("/api/library/{line}/peaks");

    assert_eq!(
        request(address, "GET", &path, &[&bia], None).await.status,
        404
    );
    assert_eq!(request(address, "GET", &path, &[], None).await.status, 401);
}
