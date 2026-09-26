//! What the server's tests share: a server to talk to, somewhere for users'
//! files, and a client ([`client`]).

#![allow(dead_code)] // Each test binary uses its own subset of these.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_render::Tools;
use scorsese_server::renders::{Quota, RenderCache};
use scorsese_server::storage::Storage;
use scorsese_server::{Files, http};
use sqlx::postgres::PgPool;
use tokio::net::TcpListener;

mod client;

#[allow(unused_imports)] // As above: each test binary uses its own subset.
pub(crate) use client::{Response, get, request, send};

/// A listener on a port the OS picked, and the address it ended up on.
pub(crate) async fn listener() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port is always available to bind");
    let address = listener
        .local_addr()
        .expect("a bound listener has an address");
    (listener, address)
}

/// A fresh, empty directory for one test, under the system's temporary one.
pub(crate) fn scratch(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "scorsese-server-{label}-{}-{unique}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a scratch directory can be made");
    directory
}

/// ffmpeg and ffprobe, which probing uploads and drawing thumbnails need.
pub(crate) fn tools() -> Tools {
    Tools::discover().expect("ffmpeg and ffprobe must be on PATH to run these tests")
}

/// Users' files in a scratch directory of their own, read with the real tools.
pub(crate) fn files(label: &str) -> Files {
    let root = scratch(label);
    Files {
        storage: Storage::new(root.join("kept"), root.join("cache")),
        tools: tools(),
        // A quota nothing in a test reaches; the eviction tests make their own.
        renders: RenderCache::new(root.join("cache"), Quota::bytes(u64::MAX)),
    }
}

/// A migrated server on `pool`, answering from the member pool exactly as
/// production does, with users' files in `files`. Runs until the test ends.
pub(crate) async fn serve_with(pool: PgPool, files: Files) -> (SocketAddr, http::AppState) {
    let (listener, address) = listener().await;
    scorsese_server::db::migrate(&pool)
        .await
        .expect("the migrations apply");
    let members = scorsese_server::db::member_pool(&pool)
        .await
        .expect("the member pool connects");
    let state = http::AppState::new(members, files);
    let router = http::router(state.clone());
    tokio::spawn(http::serve(listener, router, std::future::pending()));
    (address, state)
}

/// [`serve_with`] files of its own, for a test that uploads nothing.
pub(crate) async fn serve(pool: PgPool) -> SocketAddr {
    serve_with(pool, files("serve")).await.0
}

/// Put a library row for `sha256` in `user`'s library directly, with no file
/// behind it — for a test about what may name an item, not about the file.
/// Its id.
pub(crate) async fn hold(pool: &PgPool, user: scorsese_server::db::UserId, sha256: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, $2, 'video', 'mp4', 0, '{}') RETURNING id",
    )
    .bind(user.get())
    .bind(sha256)
    .fetch_one(pool)
    .await
    .expect("the library row is written")
}
