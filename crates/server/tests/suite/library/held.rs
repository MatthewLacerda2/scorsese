//! An upload `PATCH` holds a connection: how many one user may hold at once,
//! and how long one may sit sending nothing.

use std::net::SocketAddr;
use std::time::Duration;

use sqlx::postgres::PgPool;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::common::stream::read_until;
use crate::common::{self, send};
use crate::library::{announce_as, member, patch};

/// The length each test upload announces: more than is ever sent at once.
const LENGTH: usize = 1000;

/// Announce a file nobody will finish; its `Location`.
async fn announced(address: SocketAddr, who: &str, name: &str) -> String {
    let created = announce_as(address, who, name, &"0".repeat(64), LENGTH).await;
    assert_eq!(created.status, 201, "{}", created.body);
    created.header("location").expect("a Location").to_owned()
}

/// A `PATCH` to `location` announcing the rest of the file, of which only
/// `sent` bytes are written: the connection, left open.
async fn stalled(address: SocketAddr, who: &str, location: &str, sent: usize) -> TcpStream {
    let mut stream = TcpStream::connect(address).await.expect("listening");
    let head = format!(
        "PATCH {location} HTTP/1.1\r\nHost: test\r\n{who}\r\nTus-Resumable: 1.0.0\r\n\
         Content-Type: application/offset+octet-stream\r\nUpload-Offset: 0\r\n\
         Content-Length: {LENGTH}\r\n\r\n"
    );
    stream.write_all(head.as_bytes()).await.expect("written");
    stream.write_all(&vec![b'x'; sent]).await.expect("written");
    stream.flush().await.expect("flushed");
    stream
}

/// How many bytes the upload at `location` has, by `HEAD`.
async fn offset(address: SocketAddr, who: &str, location: &str) -> Option<String> {
    let headers = [who, "Tus-Resumable: 1.0.0"];
    let head = send(address, "HEAD", location, &headers, &[]).await;
    head.header("upload-offset").map(str::to_owned)
}

/// Wait until the upload at `location` holds `bytes`: its `PATCH` is being
/// written.
async fn holds(address: SocketAddr, who: &str, location: &str, bytes: usize) {
    for _ in 0..100 {
        if offset(address, who, location).await == Some(bytes.to_string()) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{location} never reached {bytes} bytes");
}

#[sqlx::test]
async fn a_fifth_patch_at_once_is_refused_with_429(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-held")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (_, bia) = member(&pool, "bia@example.com").await;

    let mut open = Vec::new();
    for n in 0..4 {
        let location = announced(address, &ana, &format!("{n}.mp4")).await;
        open.push(stalled(address, &ana, &location, 10).await);
        holds(address, &ana, &location, 10).await;
    }
    let fifth = announced(address, &ana, "4.mp4").await;
    let refused = patch(address, &ana, &fifth, 0, b"0123456789").await;
    assert_eq!(refused.status, 429, "{}", refused.body);
    assert_eq!(refused.header("tus-resumable"), Some("1.0.0"));
    assert_eq!(offset(address, &ana, &fifth).await.as_deref(), Some("0"));

    let theirs = announced(address, &bia, "b.mp4").await;
    let sent = patch(address, &bia, &theirs, 0, b"0123456789").await;
    assert_eq!(sent.status, 204, "another user's uploads are not counted");

    drop(open.pop());
    let mut freed = None;
    for _ in 0..100 {
        let tried = patch(address, &ana, &fifth, 0, b"0123456789").await;
        if tried.status != 429 {
            freed = Some(tried.status);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(freed, Some(204), "a closed PATCH frees its place");
}

#[sqlx::test]
async fn a_patch_sending_nothing_ends_and_the_upload_resumes(pool: PgPool) {
    let idle = Duration::from_millis(300);
    let shape = |state: scorsese_server::http::AppState| state.with_upload_idle(idle);
    let (address, _) =
        common::serve_shaped(pool.clone(), common::files("upload-idle"), shape).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let location = announced(address, &ana, "slow.mp4").await;

    let mut stream = stalled(address, &ana, &location, 400).await;
    let answer = read_until(&mut stream, "\r\n\r\n", Duration::from_secs(10)).await;
    let answer = answer.expect("a stalled PATCH is answered");
    assert!(answer.starts_with("HTTP/1.1 408"), "{answer}");
    assert!(answer.to_ascii_lowercase().contains("tus-resumable: 1.0.0"));

    assert_eq!(
        offset(address, &ana, &location).await.as_deref(),
        Some("400")
    );
    let rest = patch(address, &ana, &location, 400, &[b'y'; 100]).await;
    assert_eq!(rest.status, 204, "{}", rest.body);
    assert_eq!(rest.header("upload-offset"), Some("500"));
}
