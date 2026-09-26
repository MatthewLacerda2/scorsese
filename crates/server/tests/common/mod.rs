//! Talking to a running server the way anything outside it would: over TCP.
//!
//! Raw HTTP/1.1 rather than a client library, because a request with
//! `Connection: close` is a few lines, and an HTTP client is a dependency tree
//! `cargo deny` would have to clear for the tests alone.

#![allow(dead_code)] // Each test binary uses its own subset of these.

use std::net::SocketAddr;

use scorsese_server::http;
use sqlx::postgres::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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

/// A migrated server on `pool`, answering from the member pool exactly as
/// production does. Runs until the test ends.
pub(crate) async fn serve(pool: PgPool) -> SocketAddr {
    let (listener, address) = listener().await;
    scorsese_server::db::migrate(&pool)
        .await
        .expect("the migrations apply");
    let members = scorsese_server::db::member_pool(&pool)
        .await
        .expect("the member pool connects");
    let router = http::router(http::AppState::new(members));
    tokio::spawn(http::serve(listener, router, std::future::pending()));
    address
}

/// What came back: the status, the headers as lines, and the body.
pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) headers: Vec<String>,
    pub(crate) body: String,
}

impl Response {
    /// The first header named `name`'s value, case-insensitively.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }

    /// The body as JSON.
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).expect("the body is JSON")
    }
}

/// `method path` against `address`, with extra header lines and a JSON body.
pub(crate) async fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    headers: &[&str],
    body: Option<&serde_json::Value>,
) -> Response {
    let mut stream = TcpStream::connect(address)
        .await
        .expect("the server under test is listening");
    let body = body.map(ToString::to_string).unwrap_or_default();
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n");
    for header in headers {
        request.push_str(header);
        request.push_str("\r\n");
    }
    if !body.is_empty() {
        request.push_str("Content-Type: application/json\r\n");
    }
    request.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    stream
        .write_all(request.as_bytes())
        .await
        .expect("the request is written");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("the response is read to the end");
    let (head, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .expect("the response starts with a status line");
    Response {
        status,
        headers: lines.map(str::to_owned).collect(),
        body: body.to_owned(),
    }
}

/// `GET path` against `address`: the status code and the body.
pub(crate) async fn get(address: SocketAddr, path: &str) -> (u16, String) {
    let response = request(address, "GET", path, &[], None).await;
    (response.status, response.body)
}
