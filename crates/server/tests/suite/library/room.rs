//! What else the quota holds (#1647): a free file decided and admitted in one
//! transaction, a user's page captures counted, and every write refused while
//! the disk is below its floor.

use std::path::Path;

use scorsese_render::testing::still;
use scorsese_server::db;
use scorsese_server::events::Events;
use scorsese_server::jobs::Queue;
use scorsese_server::library::{Arrival, Kind, Library, LibraryError};
use scorsese_server::renders::Quota;
use sqlx::postgres::PgPool;

use crate::common;
use crate::library::quota::hash;
use crate::library::{announce_as, member};

/// A library whose every user may keep `quota` bytes, below the disk floor
/// `floor`, with its files under `label`.
async fn library_of(pool: &PgPool, label: &str, quota: u64, floor: u64) -> Library {
    db::migrate(pool).await.expect("the migrations apply");
    let files = common::files(label);
    Library::new(
        common::members(pool).await,
        files
            .storage
            .with_quota(Quota::bytes(quota))
            .with_floor(Quota::bytes(floor)),
        files.tools,
        Queue::new(Events::new()),
    )
}

/// A picture of `colour` under `directory`, as a free tool would have left it.
fn picture(directory: &Path, colour: &str) -> (Arrival, u64) {
    let file = directory.join(format!("{colour}.png"));
    still(&common::tools(), &file, colour, "64x36");
    let size = std::fs::metadata(&file)
        .expect("the picture is there")
        .len();
    let arrival = Arrival {
        file,
        name: format!("{colour}.png"),
        kind: Kind::Image,
        extension: "png".to_owned(),
        announced: None,
        brief_hash: None,
        words: None,
    };
    (arrival, size)
}

#[sqlx::test]
async fn two_free_files_racing_for_the_last_room_land_one(pool: PgPool) {
    let scratch = common::scratch::dir("quota-race-media");
    let (red, red_size) = picture(&scratch, "red");
    let (blue, blue_size) = picture(&scratch, "blue");
    // Room for either, not for both.
    let library = library_of(&pool, "quota-race", red_size + blue_size - 1, 0).await;
    let (ana, _) = member(&pool, "ana@example.com").await;

    let keep = |arrival: Arrival| {
        let library = library.clone();
        async move { library.admit_free(ana, arrival).await }
    };
    let (first, second) = tokio::join!(keep(red), keep(blue));

    let landed = [&first, &second].iter().filter(|kept| kept.is_ok()).count();
    assert_eq!(landed, 1, "{first:?} {second:?}");
    let refused = [first, second].into_iter().find_map(Result::err);
    assert!(
        matches!(refused, Some(LibraryError::Full { .. })),
        "{refused:?}"
    );
}

#[sqlx::test]
async fn the_page_cache_counts_toward_the_quota(pool: PgPool) {
    let files = common::files("quota-pages");
    let (ana, who) = member(&pool, "ana@example.com").await;
    let quota = 10_000;
    let pages = files.storage.page_cache(ana).join("7").join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(pages.join("frames.mkv"), vec![0; 4_000]).unwrap();
    let files = scorsese_server::Files {
        storage: files.storage.with_quota(Quota::bytes(quota)),
        ..files
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
    let files = common::files("quota-floor");
    let files = scorsese_server::Files {
        // No disk has this much free.
        storage: files.storage.with_floor(Quota::bytes(u64::MAX)),
        ..files
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
