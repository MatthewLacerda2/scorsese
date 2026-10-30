//! What the server's tests share: a server to talk to, somewhere for users'
//! files, and a client ([`client`]).

use std::net::SocketAddr;

use scorsese_server::renders::{Quota, RenderCache};
use scorsese_server::storage::Storage;
use scorsese_server::{Files, http};
use sqlx::postgres::PgPool;
use tokio::net::TcpListener;

pub(crate) mod capturing;
mod client;
pub(crate) mod logged;
#[path = "../../../../../tools/scratch.rs"]
pub(crate) mod scratch;
pub(crate) mod stream;
mod teardown;

pub(crate) use client::{Response, get, request, send};
pub(crate) use teardown::{members, release_at_teardown};

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

/// ffmpeg and ffprobe, which probing uploads and drawing thumbnails need.
#[allow(unused_imports)] // As above.
pub(crate) use scorsese_render::testing::tools;

/// Users' files in a scratch directory of their own, read with the real tools.
pub(crate) fn files(label: &str) -> Files {
    let root = scratch::dir(label);
    Files {
        storage: Storage::new(root.join("kept"), root.join("cache")),
        tools: tools(),
        // A quota nothing in a test reaches; the eviction tests make their own.
        renders: RenderCache::new(root.join("cache"), Quota::bytes(u64::MAX)),
    }
}

/// A migrated server on `pool`, answering from the member pool exactly as
/// production does, with users' files in `files`. Runs until the test ends,
/// and lets go of the database then ([`teardown`]).
pub(crate) async fn serve_with(pool: PgPool, files: Files) -> (SocketAddr, http::AppState) {
    serve_shaped(pool, files, |state| state).await
}

/// [`serve_with`], its state first passed through `shape`.
pub(crate) async fn serve_shaped(
    pool: PgPool,
    files: Files,
    shape: impl FnOnce(http::AppState) -> http::AppState,
) -> (SocketAddr, http::AppState) {
    let (listener, address) = listener().await;
    scorsese_server::db::migrate(&pool)
        .await
        .expect("the migrations apply");
    let members = scorsese_server::db::member_pool(&pool)
        .await
        .expect("the member pool connects");
    let state = shape(http::AppState::new(members.clone(), files));
    let router = http::router(state.clone());
    let server = tokio::spawn(http::serve(listener, router, std::future::pending()));
    release_at_teardown(&pool, &members, [server.abort_handle()]);
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

/// A MIDI file: format 0, 96 ticks a beat, C4 then E4 for a beat each —
/// written byte by byte, a real one, so the web refusing it (#964) is about
/// the kind and not about the bytes.
pub(crate) const TUNE: &[u8] = &[
    b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, //
    b'M', b'T', b'r', b'k', 0, 0, 0, 20, //
    0x00, 0x90, 60, 100, 0x60, 0x80, 60, 0, //
    0x00, 0x90, 64, 100, 0x60, 0x80, 64, 0, //
    0x00, 0xFF, 0x2F, 0x00,
];
