//! Media a tool downloaded into a laid-out project — `stock_import` (#900) —
//! kept in its owner's library before the document naming it is saved.

use scorsese_core::{Fps, Project, Source, StockLibrary, import_path};
use scorsese_render::Ffprobe;
use scorsese_render::testing::still;
use scorsese_server::db;
use scorsese_server::events::Events;
use scorsese_server::jobs::Queue;
use scorsese_server::library::Library;
use scorsese_server::tools::keep_fetched;
use sqlx::postgres::PgPool;

use crate::common;
use crate::library::member;

async fn library(pool: &PgPool) -> Library {
    db::migrate(pool).await.expect("the migrations apply");
    let files = common::files("fetched");
    let members = common::members(pool).await;
    Library::new(
        members,
        files.storage,
        files.tools,
        Queue::new(Events::new()),
    )
}

/// How many of the library's items hold `sha256`.
async fn held(pool: &PgPool, sha256: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM library_items WHERE sha256 = $1")
        .bind(sha256)
        .fetch_one(pool)
        .await
        .expect("the library answers")
}

#[sqlx::test]
async fn a_downloaded_picture_is_admitted_once(pool: PgPool) {
    let library = library(&pool).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let scratch = common::scratch::dir("fetched-picture");
    let picture = scratch.join("pixabay-7.png");
    still(&common::tools(), &picture, "red", "64x36");

    let root = scratch.join("laid.scor");
    let before = Project::create(&root, Some("laid"), Fps::THIRTY, scorsese_core::Ratio::WIDE)
        .expect("a project");
    let mut after = before.clone();
    let probe = Ffprobe::new(common::tools());
    import_path(&mut after, &root, &picture, None, &probe).expect("the picture imports");
    let sha256 = after.assets[0].sha256.clone().expect("it is hashed");

    keep_fetched(&library, ana, &before, &after, &root)
        .await
        .expect("it is kept");
    assert_eq!(held(&pool, &sha256).await, 1);
    // What was already the project's is never admitted again.
    keep_fetched(&library, ana, &after, &after, &root)
        .await
        .expect("nothing to keep");
    assert_eq!(held(&pool, &sha256).await, 1);
}

#[sqlx::test]
async fn a_stock_file_s_source_rides_on_its_item(pool: PgPool) {
    let library = library(&pool).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let scratch = common::scratch::dir("fetched-source");
    let picture = scratch.join("pixabay-7.png");
    still(&common::tools(), &picture, "red", "64x36");

    let root = scratch.join("laid.scor");
    let before = Project::create(&root, Some("laid"), Fps::THIRTY, scorsese_core::Ratio::WIDE)
        .expect("a project");
    let mut after = before.clone();
    let probe = Ffprobe::new(common::tools());
    import_path(&mut after, &root, &picture, None, &probe).expect("the picture imports");
    let source = Source {
        library: StockLibrary::Pixabay,
        id: "7".to_owned(),
        author: Some("someone".to_owned()),
        page_url: Some("https://pixabay.com/photos/id-7/".to_owned()),
        licence: Some("Pixabay Content License".to_owned()),
    };
    after.assets[0].source = Some(source.clone());

    keep_fetched(&library, ana, &before, &after, &root)
        .await
        .expect("it is kept");
    let sha256 = after.assets[0].sha256.clone().expect("it is hashed");
    let id: i64 = sqlx::query_scalar("SELECT id FROM library_items WHERE sha256 = $1")
        .bind(&sha256)
        .fetch_one(&pool)
        .await
        .expect("the item is there");
    let kept = library.source(ana, id).await.expect("the library answers");
    assert_eq!(kept, Some(source));
}

#[sqlx::test]
async fn a_stock_file_past_the_quota_is_refused_in_plain_words(pool: PgPool) {
    db::migrate(&pool).await.unwrap();
    let files = common::files("fetched-full");
    let tiny = scorsese_server::renders::Quota::bytes(10);
    let members = common::members(&pool).await;
    let queue = Queue::new(Events::new());
    let library = Library::new(members, files.storage.with_quota(tiny), files.tools, queue);
    let (ana, _) = member(&pool, "ana@example.com").await;
    let scratch = common::scratch::dir("fetched-full-picture");
    let picture = scratch.join("pixabay-8.png");
    still(&common::tools(), &picture, "blue", "64x36");
    let root = scratch.join("laid.scor");
    let before =
        Project::create(&root, Some("laid"), Fps::THIRTY, scorsese_core::Ratio::WIDE).unwrap();
    let mut after = before.clone();
    let probe = Ffprobe::new(common::tools());
    import_path(&mut after, &root, &picture, None, &probe).unwrap();

    let refused = keep_fetched(&library, ana, &before, &after, &root)
        .await
        .unwrap_err();
    assert!(refused.contains("your storage is full"), "{refused}");
    let sha256 = after.assets[0].sha256.clone().unwrap();
    assert_eq!(held(&pool, &sha256).await, 0);
}
