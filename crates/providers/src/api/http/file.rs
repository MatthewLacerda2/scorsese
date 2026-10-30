//! Streaming a public file to disk: the one download larger than any media
//! reply, the page renderer `chromium` installs — and a stock file, whose
//! every hop is checked ([`stream_where`]).

use std::io::{Read, Write};
use std::time::Duration;

use super::{HttpError, TIMEOUT, refused};

/// How long a file download may take whole — the page renderer's ~100 MB on a
/// slow line. Generous for the same reason [`STREAM_WHOLE`](super::STREAM_WHOLE) is: what it bounds is
/// a transfer that has genuinely stalled, not one that is merely slow.
const FILE_WHOLE: Duration = Duration::from_secs(60 * 60);

/// The most redirects [`stream_where`] follows.
const HOPS: usize = 5;

/// GETs `url` — a public file, so nothing is signed — and streams the reply
/// into `to`, telling `progress` how many bytes have arrived and how many the
/// server said to expect. Answers how many arrived in all.
///
/// Streamed rather than read into memory like [`Caller::download`](super::Caller::download), because
/// this is for files larger than any media reply: the pinned browser the
/// `chromium` module installs. `limit` still bounds it, for the same reason.
pub fn stream_to(
    url: &str,
    limit: u64,
    to: &mut dyn Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64, HttpError> {
    let response = refused(url, agent(None).get(url).call())?;
    copy(url, response, limit, to, progress)
}

/// [`stream_to`], following a redirect only to where `allowed` says it may
/// go, and at most `HOPS` of them: a redirect anywhere else is refused as
/// the redirect it is, its body unread.
pub fn stream_where(
    url: &str,
    limit: u64,
    to: &mut dyn Write,
    allowed: &dyn Fn(&str) -> bool,
) -> Result<u64, HttpError> {
    let agent = agent(Some(0));
    let mut at = url.to_owned();
    for _ in 0..=HOPS {
        let sent = agent.get(&at).call();
        let response = sent.map_err(|error| HttpError::Unreachable {
            url: at.clone(),
            message: error.to_string(),
        })?;
        let status = response.status().as_u16();
        if !(300..400).contains(&status) {
            let response = refused(&at, Ok(response))?;
            return copy(&at, response, limit, to, &mut |_, _| {});
        }
        let next = response
            .headers()
            .get("location")
            .and_then(|location| location.to_str().ok())
            .and_then(|location| joined(&at, location))
            .filter(|next| allowed(next));
        match next {
            Some(next) => at = next,
            None => {
                return Err(HttpError::Refused {
                    url: at,
                    status,
                    body: String::new(),
                });
            }
        }
    }
    Err(HttpError::Refused {
        url: at,
        status: 310,
        body: String::from("too many redirects"),
    })
}

/// The agent a file is fetched with: following ureq's own redirects, or at
/// most `redirects` of them.
fn agent(redirects: Option<u32>) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(FILE_WHOLE))
        .timeout_recv_response(Some(TIMEOUT))
        .http_status_as_error(false);
    match redirects {
        Some(most) => config.max_redirects(most).build().into(),
        None => config.build().into(),
    }
}

/// Where a redirect from `from` to `location` leads: an absolute address, or
/// one on `from`'s own host. Anything else is no address to follow.
fn joined(from: &str, location: &str) -> Option<String> {
    if location.starts_with("https://") || location.starts_with("http://") {
        return Some(location.to_owned());
    }
    let parsed: ureq::http::Uri = from.parse().ok()?;
    let scheme = parsed.scheme_str()?;
    if let Some(rest) = location.strip_prefix("//") {
        return Some(format!("{scheme}://{rest}"));
    }
    let authority = parsed.authority()?.as_str();
    location
        .starts_with('/')
        .then(|| format!("{scheme}://{authority}{location}"))
}

/// The body of `response` from `url`, at most `limit` bytes, into `to`.
fn copy(
    url: &str,
    mut response: ureq::http::Response<ureq::Body>,
    limit: u64,
    to: &mut dyn Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64, HttpError> {
    let total = response.body().content_length();
    let unreadable = |error: std::io::Error| HttpError::Unreadable {
        url: url.to_owned(),
        message: error.to_string(),
    };
    let mut reader = response.body_mut().as_reader().take(limit);
    let mut buffer = vec![0; 256 * 1024];
    let mut received = 0;
    progress(received, total);
    loop {
        let read = reader.read(&mut buffer).map_err(unreadable)?;
        if read == 0 {
            return Ok(received);
        }
        to.write_all(&buffer[..read]).map_err(unreadable)?;
        received += read as u64;
        progress(received, total);
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    use super::{joined, stream_where};

    /// A server on loopback answering each of `answers` to one request in
    /// turn, and its address.
    fn serving(answers: Vec<String>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = format!("http://{}", listener.local_addr().expect("bound"));
        std::thread::spawn(move || {
            for answer in answers {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                    line.clear();
                }
                let _ = stream.write_all(answer.as_bytes());
            }
        });
        address
    }

    fn moved(to: &str) -> String {
        format!("HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\n\r\n")
    }

    const FILE: &str = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nfile";

    #[test]
    fn a_redirect_where_it_may_go_is_followed() {
        let address = serving(vec![moved("/there"), FILE.to_owned()]);
        let mut got = Vec::new();
        let read = stream_where(&format!("{address}/here"), 64, &mut got, &|_| true);
        assert_eq!(read.expect("followed"), 4);
        assert_eq!(got, b"file");
    }

    #[test]
    fn a_redirect_anywhere_else_is_refused_unread() {
        let address = serving(vec![moved("http://127.0.0.1:9/secret")]);
        let mut got = Vec::new();
        let first = format!("{address}/here");
        let refused =
            stream_where(&first, 64, &mut got, &|url| url == first).expect_err("not followed");
        assert_eq!(refused.status(), Some(302));
        assert!(got.is_empty());
    }

    #[test]
    fn a_location_is_absolute_or_on_the_same_host() {
        let from = "https://pixabay.com/get/x";
        assert_eq!(joined(from, "/y").as_deref(), Some("https://pixabay.com/y"));
        assert_eq!(
            joined(from, "//cdn.pixabay.com/z").as_deref(),
            Some("https://cdn.pixabay.com/z")
        );
        assert_eq!(
            joined(from, "http://elsewhere/").as_deref(),
            Some("http://elsewhere/")
        );
        assert_eq!(joined(from, "relative"), None);
    }
}
