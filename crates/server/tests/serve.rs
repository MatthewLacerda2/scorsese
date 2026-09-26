//! The server starts, answers its health check, and stops when told to.

mod common;

use std::time::Duration;

use scorsese_providers::credentials::Secret;
use scorsese_server::accounts::{tokens, users};
use scorsese_server::jobs::Registry;
use scorsese_server::{Config, ServerError, db, http};
use sqlx::postgres::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::time::timeout;

/// Long enough for any healthy stop; short enough that a hang fails the test
/// instead of the whole run.
const STOP: Duration = Duration::from_secs(10);

#[sqlx::test(migrations = false)]
async fn it_migrates_serves_and_stops_cleanly(pool: PgPool) {
    let (listener, address) = common::listener().await;
    let (stop, stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(scorsese_server::start(
        pool.clone(),
        listener,
        common::files("serve"),
        Registry::new(),
        async {
            stopped.await.ok();
        },
    ));

    let (status, body) = common::get(address, "/api/health").await;
    assert_eq!((status, body.as_str()), (200, "ok"));

    // Migrations ran before the first request was answered.
    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(applied, db::MIGRATOR.iter().count() as i64);

    stop.send(()).unwrap();
    let outcome = timeout(STOP, server).await.expect("the server stopped");
    outcome.unwrap().unwrap();
}

/// Port 1 on loopback: nothing listens there, so every connect is refused.
/// The two tests below need no database — they are about it being gone.
const NOWHERE: &str = "postgres://nobody@127.0.0.1:1/nothing";

#[tokio::test]
async fn a_database_it_cannot_reach_is_a_startup_error() {
    let config = Config {
        database_url: Secret::new(NOWHERE),
        storage: std::env::temp_dir().join("scorsese-server-never-created"),
        cache: std::env::temp_dir().join("scorsese-server-cache-never-created"),
        bind: "127.0.0.1:0".parse().unwrap(),
    };
    let outcome = timeout(STOP, scorsese_server::run(config, std::future::pending()))
        .await
        .expect("an unreachable database fails promptly");
    assert!(
        matches!(outcome, Err(ServerError::Connect(_))),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn health_says_unavailable_when_the_database_is_unreachable() {
    let pool = db::options()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy(NOWHERE)
        .unwrap();
    let (listener, address) = common::listener().await;
    let router = http::router(http::AppState::new(pool, common::files("health")));
    let server = tokio::spawn(http::serve(listener, router, std::future::pending()));

    let (status, body) = common::get(address, "/api/health").await;
    assert_eq!((status, body.as_str()), (503, "database unreachable"));
    server.abort();
}

#[sqlx::test(migrations = false)]
async fn an_open_event_stream_does_not_hold_the_server_up(pool: PgPool) {
    let (listener, address) = common::listener().await;
    let (stop, stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(scorsese_server::start(
        pool.clone(),
        listener,
        common::files("serve"),
        Registry::new(),
        async {
            stopped.await.ok();
        },
    ));
    common::get(address, "/api/health").await; // migrated by now
    let ana = users::create(&pool, "ana@example.com", "password one")
        .await
        .unwrap();
    let token = tokens::issue(&pool, ana, "t").await.unwrap().token;

    // A browser listening, which never hangs up by itself.
    let mut stream = TcpStream::connect(address).await.unwrap();
    let request =
        format!("GET /api/events HTTP/1.1\r\nHost: t\r\nAuthorization: Bearer {token}\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut head = [0u8; 12];
    stream.read_exact(&mut head).await.unwrap();
    assert_eq!(&head, b"HTTP/1.1 200");

    stop.send(()).unwrap();
    let outcome = timeout(STOP, server).await.expect("the server stopped");
    outcome.unwrap().unwrap();
}
