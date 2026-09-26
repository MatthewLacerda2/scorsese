//! Talking to a running server the way anything outside it would: over TCP.
//!
//! Raw HTTP/1.1 rather than a client library, because a request with
//! `Connection: close` is a few lines, and an HTTP client is a dependency tree
//! `cargo deny` would have to clear for the tests alone.

use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// What came back: the status, the headers as lines, and the body — as text,
/// and as the bytes it was.
pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) headers: Vec<String>,
    pub(crate) body: String,
    pub(crate) bytes: Vec<u8>,
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
    let body = body.map(ToString::to_string).unwrap_or_default();
    let mut lines = headers.to_vec();
    if !body.is_empty() {
        lines.push("Content-Type: application/json");
    }
    send(address, method, path, &lines, body.as_bytes()).await
}

/// `method path` against `address`, with these header lines and these bytes
/// as the body.
pub(crate) async fn send(
    address: SocketAddr,
    method: &str,
    path: &str,
    headers: &[&str],
    body: &[u8],
) -> Response {
    let mut stream = TcpStream::connect(address)
        .await
        .expect("the server under test is listening");
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n");
    for header in headers {
        request.push_str(header);
        request.push_str("\r\n");
    }
    request.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    let mut bytes = request.into_bytes();
    bytes.extend_from_slice(body);
    stream
        .write_all(&bytes)
        .await
        .expect("the request is written");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("the response is read to the end");
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or(response.len());
    let head = String::from_utf8_lossy(&response[..split]).into_owned();
    let body = response.get(split + 4..).unwrap_or_default().to_vec();
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .expect("the response starts with a status line");
    Response {
        status,
        headers: lines.map(str::to_owned).collect(),
        body: String::from_utf8_lossy(&body).into_owned(),
        bytes: body,
    }
}

/// `GET path` against `address`: the status code and the body.
pub(crate) async fn get(address: SocketAddr, path: &str) -> (u16, String) {
    let response = request(address, "GET", path, &[], None).await;
    (response.status, response.body)
}
