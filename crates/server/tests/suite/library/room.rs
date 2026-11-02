//! What else the quota holds (#1647): a user's page captures counted, and
//! every write refused while the disk is below its floor. Written against
//! names the base already had, so CI's regression job can build it there;
//! the race between two free files is `library::room`'s own test.

use scorsese_providers::credentials::Environment;
use scorsese_server::captures::{SPOOL_DIR, Spool};
use scorsese_server::config::{CACHE, DATABASE_URL, STORAGE};
use scorsese_server::renders::Quota;
use scorsese_server::storage::Storage;
use scorsese_server::{Config, Files};
use sqlx::postgres::PgPool;

use crate::common;
use crate::library::quota::hash;
use crate::library::{announce_as, member};

#[sqlx::test]
async fn the_page_cache_counts_toward_the_quota(pool: PgPool) {
    let root = common::scratch::dir("quota-pages");
    let cache = root.join("cache");
    let (ana, who) = member(&pool, "ana@example.com").await;
    let quota = 10_000;
    let pages = Spool::new(cache.join(SPOOL_DIR))
        .pages(ana, 7)
        .join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(pages.join("frames.mkv"), vec![0; 4_000]).unwrap();
    let files = Files {
        storage: Storage::new(root.join("kept"), cache).with_quota(Quota::bytes(quota)),
        ..common::files("quota-pages-tools")
    };
    let (address, _) = common::serve_with(pool.clone(), files).await;

    let over = announce_as(address, &who, "long.mp4", &hash('a'), 6_001).await;
    assert_eq!(over.status, 413, "{}", over.body);
    assert!(over.body.contains("page captures"), "{}", over.body);
    let fits = announce_as(address, &who, "long.mp4", &hash('a'), 6_000).await;
    assert_eq!(fits.status, 201, "{}", fits.body);
}

#[sqlx::test]
async fn below_the_disk_floor_an_upload_is_refused(pool: PgPool) {
    let root = common::scratch::dir("quota-floor");
    let config = Config::from_environment(&Environment::of([
        (
            DATABASE_URL,
            "postgres://nobody@127.0.0.1:1/nothing".to_owned(),
        ),
        (STORAGE, root.join("kept").display().to_string()),
        (CACHE, root.join("cache").display().to_string()),
        // A billion gigabytes: no disk has this much free.
        ("SCORSESE_DISK_FLOOR_GB", "1000000000".to_owned()),
    ]))
    .expect("the settings are whole");
    let files = Files {
        storage: config.files(),
        ..common::files("quota-floor-tools")
    };
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (_, who) = member(&pool, "ana@example.com").await;

    let refused = announce_as(address, &who, "intro.mp4", &hash('a'), 1_000).await;
    assert_eq!(refused.status, 507, "{}", refused.body);
    assert!(
        refused.body.contains("nearly out of disk space"),
        "{}",
        refused.body
    );
}
