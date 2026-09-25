//! Talking to a running server the way anything outside it would: over TCP.
//!
//! Raw HTTP/1.1 rather than a client library, because one GET with
//! `Connection: close` is a few lines, and an HTTP client is a dependency tree
//! `cargo deny` would have to clear for the tests alone.

use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// A listener on a port the OS picked, and the address it ended up on.
pub async fn listener() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port is always available to bind");
    let address = listener
        .local_addr()
        .expect("a bound listener has an address");
    (listener, address)
}

/// `GET path` against `address`: the status code and the body.
pub async fn get(address: SocketAddr, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(address)
        .await
        .expect("the server under test is listening");
    let request = format!("GET {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("the request is written");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("the response is read to the end");
    let status = response
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("the response starts with a status line");
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}
