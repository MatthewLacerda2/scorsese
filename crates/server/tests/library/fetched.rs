//! Media a tool downloaded into a laid-out project — `stock_import` (#900) —
//! kept in its owner's library before the document naming it is saved.

use scorsese_core::{Fps, Project, import_path};
use scorsese_render::Ffprobe;
use scorsese_server::db;
use scorsese_server::events::Events;
use scorsese_server::jobs::Queue;
use scorsese_server::library::Library;
use scorsese_server::tools::keep_fetched;
use sqlx::postgres::PgPool;

use crate::common;
use crate::member;

async fn library(pool: &PgPool) -> Library {
    db::migrate(pool).await.expect("the migrations apply");
    let files = common::files("fetched");
    let members = db::member_pool(pool)
        .await
        .expect("the member pool connects");
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
    let scratch = common::scratch("fetched-picture");
    let picture = scratch.join("pixabay-7.png");
    let output = common::tools()
        .ffmpeg()
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=64x36",
        ])
        .args(["-frames:v", "1"])
        .arg(&picture)
        .output()
        .expect("ffmpeg runs");
    assert!(output.status.success(), "{output:?}");

    let root = scratch.join("laid.scor");
    let before = Project::create(&root, Some("laid"), Fps::THIRTY).expect("a project");
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
