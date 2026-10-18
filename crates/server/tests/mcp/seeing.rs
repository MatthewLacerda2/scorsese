//! A still of a page on the web (#1434): captured by the capture worker, the
//! way a render's pages are, and never by a browser in the server's process —
//! which on the hosted service cannot start one, and so drew every page as
//! its slug card.
//!
//! The worker runs the real browser (`SCORSESE_CHROME`) through the real
//! binary's `capture-one`, its sandbox off as `tests/captures` has it.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use scorsese_server::captures::Spool;
use scorsese_server::captures::worker::{Isolation, Worker};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{call, common, member};

/// A project showing the page `title` for its first second.
fn showing_a_page() -> Value {
    json!({
        "assets": [{ "id": "title", "kind": "html", "path": "pages/title.html" }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c", "asset": "title", "start": 0, "duration": 30 }
        ]}]
    })
}

/// A server, its cache made as the binary makes it, and its spool.
async fn serving(pool: &PgPool, label: &str) -> (std::net::SocketAddr, Spool) {
    let files = common::files(label);
    files
        .renders
        .prepare()
        .expect("the cache and its spool are made");
    let spool = files.renders.captures();
    (common::serve_with(pool.clone(), files).await.0, spool)
}

/// A worker answering `spool` for the rest of the test, once it has said so.
fn answering(spool: &Spool) {
    let worker = Worker {
        spool: spool.clone(),
        isolation: Isolation::Process {
            program: PathBuf::from(env!("CARGO_BIN_EXE_scorsese-server")),
            sandbox: false,
            library: None,
        },
        deadline: scorsese_server::captures::deadline,
    };
    std::thread::spawn(move || worker.run());
    let alive = spool.root().join("worker.alive");
    let started = Instant::now();
    while !alive.is_file() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the worker beats"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// `user`'s still of the page at half a second, after writing it.
async fn still_of_the_page(address: std::net::SocketAddr, token: &str, id: i64) -> String {
    let page = "<body style='margin:0;background:#0f0'></body>";
    let written = json!({ "project": id, "page": "title", "html": page });
    let (said, refused) = call(address, token, "page_write", written).await;
    assert!(!refused, "{said}");
    let look = json!({ "project": id, "at": "0.5s", "resolution": "64x36" });
    let (said, refused) = call(address, token, "still", look).await;
    assert!(!refused, "{said}");
    said
}

#[sqlx::test]
async fn a_still_of_a_page_is_captured_by_the_worker_and_shows_the_page(pool: PgPool) {
    let (address, spool) = serving(&pool, "still-captured").await;
    answering(&spool);
    let (ana, token) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, showing_a_page()).await;

    let said = still_of_the_page(address, &token, id).await;

    assert!(!said.contains("could not be captured"), "{said}");
    assert!(!said.contains("note:"), "{said}");
    let jobs = std::fs::read_dir(spool.jobs()).map_or(0, |jobs| jobs.count());
    assert_eq!(jobs, 0, "the still's job is gone once answered");

    // Kept in the project's page cache: the next look asks for nothing.
    let look = json!({ "project": id, "at": "0.5s", "resolution": "64x36" });
    let (again, refused) = call(address, &token, "still", look).await;
    assert!(!refused && !again.contains("note:"), "{again}");
}

#[sqlx::test]
async fn a_still_with_no_worker_says_so_and_never_starts_a_browser(pool: PgPool) {
    let (address, _spool) = serving(&pool, "still-unanswered").await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, showing_a_page()).await;

    let started = Instant::now();
    let said = still_of_the_page(address, &token, id).await;

    // With nobody to capture it, a page drawn anyway was drawn by a browser
    // in this process: what the server did before #1434, wherever the
    // sandbox could start (here, `SCORSESE_CHROME_NO_SANDBOX`).
    assert!(said.contains("could not be captured"), "{said}");
    assert!(said.contains("not running on this server"), "{said}");
    assert!(!said.contains("sandbox"), "{said}");
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "and at once, without waiting for a worker that is not there"
    );
}
