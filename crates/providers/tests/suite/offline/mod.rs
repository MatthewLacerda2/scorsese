//! The test suite runs with the network refused (#1302), and loopback open.
//!
//! `.config/nextest.toml`'s `offline` setup script points every proxy variable
//! at a port where nothing listens; the transport every vendor call goes
//! through reads them. These hold both halves of that, through the same
//! [`Caller`] a vendor module uses: a request for the internet is refused on
//! this machine, and a server a test runs on loopback is still reached.
//!
//! Outside nextest (`cargo test`) the variables are not set, and the first
//! test says so rather than letting a request out to find out.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

use scorsese_providers::api::http::{Caller, HttpError};

/// Where the setup script sends outbound HTTP.
const DEAD_PROXY: &str = "http://127.0.0.1:9";

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "the proxy nextest's offline script sets is what this test checks, not a secret"
)]
fn a_request_for_the_internet_is_refused_before_it_leaves() {
    for name in ["ALL_PROXY", "HTTPS_PROXY", "HTTP_PROXY"] {
        assert_eq!(
            std::env::var(name).as_deref(),
            Ok(DEAD_PROXY),
            "{name} is not the dead proxy: the suite runs through nextest \
             (`make test`), whose `offline` setup script sets it"
        );
    }

    let answer = Caller::anonymous().download("https://example.com/", 1024);

    // Unreachable, refused: the transport failed here. An answer of any kind,
    // even a refusal from the server, would mean the request left the machine.
    match answer {
        Err(HttpError::Unreachable { message, .. }) => {
            assert!(message.contains("refused"), "not a refusal: {message}");
        }
        other => panic!("the request was not refused on this machine: {other:?}"),
    }
}

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "the loopback list nextest's offline script sets is what this test checks, not a secret"
)]
fn every_name_for_loopback_goes_direct() {
    for list in ["NO_PROXY", "no_proxy"] {
        let hosts = std::env::var(list).unwrap_or_default();
        for host in ["localhost", "127.0.0.1", "::1"] {
            assert!(
                hosts.split(',').any(|h| h == host),
                "{host} not in {list}={hosts}"
            );
        }
    }
}

#[test]
fn a_server_on_loopback_is_still_reached() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        // The request's head ends at its first blank line; a GET has no body.
        while reader.read_line(&mut line).unwrap() > 0 && line != "\r\n" {
            line.clear();
        }
        let body = b"loopback";
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut stream = reader.into_inner();
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
    });

    let got = Caller::anonymous().download(&format!("http://127.0.0.1:{port}/"), 1024);
    assert_eq!(got.unwrap(), b"loopback");
    server.join().unwrap();
}
