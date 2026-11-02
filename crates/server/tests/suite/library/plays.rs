//! How long an uploaded file may play (#1634): three hours, by its probe,
//! whatever its size — a small file can play for a very long time.

use scorsese_render::testing::generate;
use scorsese_server::renders::{Quota, RenderCache};
use scorsese_server::storage::Storage;
use sqlx::postgres::PgPool;

use crate::common::{self, request};
use crate::library::{announce, member, patch, upload};

/// A video `seconds` long at a frame a second and sixteen pixels a side: a
/// few hundred kilobytes, whatever it plays for.
fn lasting(name: &str, seconds: u64) -> Vec<u8> {
    let file = common::scratch::dir(name).join("long.mkv");
    let source = format!("testsrc=s=16x16:r=1:d={seconds}");
    generate(
        &common::tools(),
        &file,
        &[
            "-f",
            "lavfi",
            "-i",
            &source,
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
        ],
    );
    std::fs::read(file).expect("the fixture reads")
}

#[sqlx::test]
async fn a_file_that_plays_three_hours_is_kept(pool: PgPool) {
    let bytes = lasting("plays-three-hours", 3 * 60 * 60);
    let (address, _) = common::serve_with(pool.clone(), common::files("plays-three")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    upload(address, &ana, "long.mkv", &bytes).await;
}

#[sqlx::test]
async fn a_file_that_plays_longer_is_refused_and_not_kept(pool: PgPool) {
    let bytes = lasting("plays-longer", 3 * 60 * 60 + 1);
    let root = common::scratch::dir("plays-longer-files");
    let files = scorsese_server::Files {
        storage: Storage::new(root.join("kept"), root.join("cache")),
        tools: common::tools(),
        renders: RenderCache::new(root.join("cache"), Quota::bytes(u64::MAX)),
    };
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let created = announce(address, &ana, "long.mkv", &bytes).await;
    assert_eq!(created.status, 201, "{}", created.body);
    let location = created.header("location").unwrap().to_owned();
    let refused = patch(address, &ana, &location, 0, &bytes).await;
    assert_eq!(refused.status, 422, "{}", refused.body);
    assert!(
        refused
            .body
            .contains("at most 3 hours; this one plays 3 h 0 min 1 s"),
        "{}",
        refused.body
    );
    let listed = request(address, "GET", "/api/library", &[&ana], None).await;
    assert_eq!(
        listed.json(),
        serde_json::json!([]),
        "nothing refused is kept"
    );
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM uploads")
        .fetch_one(&pool)
        .await
        .expect("the uploads count");
    assert_eq!(left, 0, "the upload counts against nothing");
    let kept = walk(&root);
    assert!(kept.is_empty(), "no bytes are left behind: {kept:?}");
}

/// Every file under `root`.
fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found
}
