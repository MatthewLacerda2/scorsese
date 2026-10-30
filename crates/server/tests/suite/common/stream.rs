//! `GET /api/events` held open the way a browser holds it: one connection,
//! read as it is written.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// `GET /api/events` with `credential` (an `Authorization:` or `Cookie:`
/// line): the connection, its `200` head already read.
pub(crate) async fn listen(address: SocketAddr, credential: &str) -> TcpStream {
    let mut stream = TcpStream::connect(address)
        .await
        .expect("the server is listening");
    let request = format!("GET /api/events HTTP/1.1\r\nHost: test\r\n{credential}\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("the request is written");
    let head = read_until(&mut stream, "\r\n\r\n", Duration::from_secs(5)).await;
    assert!(
        head.expect("the stream answers")
            .starts_with("HTTP/1.1 200")
    );
    stream
}

/// What `stream` says up to and including `marker`, or `None` if it has not
/// said it within `patience` (or the connection closed first).
pub(crate) async fn read_until(
    stream: &mut TcpStream,
    marker: &str,
    patience: Duration,
) -> Option<String> {
    let mut seen = Vec::new();
    let reading = async {
        let mut byte = [0u8; 1];
        while !String::from_utf8_lossy(&seen).contains(marker) {
            match stream.read(&mut byte).await {
                Ok(1) => seen.push(byte[0]),
                _ => return false,
            }
        }
        true
    };
    match timeout(patience, reading).await {
        Ok(true) => Some(String::from_utf8_lossy(&seen).into_owned()),
        _ => None,
    }
}

/// Whether the response on `stream` ends within `patience`: its chunked
/// body's last, empty chunk arrives (keep-alive comments before it are
/// skipped), or the connection closes.
pub(crate) async fn ends(stream: &mut TcpStream, patience: Duration) -> bool {
    let mut seen = Vec::new();
    let reading = async {
        let mut byte = [0u8; 1];
        loop {
            match stream.read(&mut byte).await {
                Ok(1) => seen.push(byte[0]),
                _ => return,
            }
            if seen.ends_with(b"\r\n0\r\n\r\n") || seen == b"0\r\n\r\n" {
                return;
            }
        }
    };
    timeout(patience, reading).await.is_ok()
}
