//! Streaming a public file to disk: the one download larger than any media
//! reply, the page renderer `chromium` installs.

use std::io::{Read, Write};
use std::time::Duration;

use super::{HttpError, TIMEOUT, refused};

/// How long a file download may take whole — the page renderer's ~100 MB on a
/// slow line. Generous for the same reason [`STREAM_WHOLE`](super::STREAM_WHOLE) is: what it bounds is
/// a transfer that has genuinely stalled, not one that is merely slow.
const FILE_WHOLE: Duration = Duration::from_secs(60 * 60);

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
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(FILE_WHOLE))
        .timeout_recv_response(Some(TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = refused(url, agent.get(url).call())?;
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
