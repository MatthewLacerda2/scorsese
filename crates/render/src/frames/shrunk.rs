//! A picture made small enough to send a model (#1435).
//!
//! A frame is written as PNG because PNG is lossless (the parent module says
//! why that matters for a file). A model looking at it needs none of that: it
//! downsamples anything past [`MODEL_EDGE`] on the long edge before it sees a
//! pixel, and a 1280×720 PNG of a photographic frame is 2–3 MB of base64 —
//! replayed with every later turn of a conversation, until the vendor refuses
//! the request for its size. A JPEG at that edge is a tenth of it or less, and
//! shows the model the same picture.
//!
//! Through ffmpeg like every other picture here, over pipes, so nothing is
//! left on disk.

use std::io::Write;
use std::path::Path;
use std::process::Stdio;

use super::{FrameError, stderr_of};
use crate::tools::Tools;

/// The longest edge, in pixels, a picture for a model keeps. Anthropic's own
/// guidance: larger images are scaled down to it before the model reads them.
pub const MODEL_EDGE: u32 = 1568;

/// ffmpeg's `-q:v` for the JPEG: about quality 85 on the usual 0–100 scale,
/// where text in a frame stays crisp.
const QUALITY: &str = "4";

/// What a picture in memory is called in an error, having no file name.
const IN_MEMORY: &str = "(a picture being shrunk for the model)";

/// The media type [`for_a_model`] writes.
pub const JPEG: &str = "image/jpeg";

/// `picture` (any still image ffmpeg reads — a PNG, in practice) as a JPEG
/// whose long edge is at most [`MODEL_EDGE`]. Never enlarged; the aspect is
/// kept.
pub fn for_a_model(tools: &Tools, picture: &[u8]) -> Result<Vec<u8>, FrameError> {
    let file = Path::new(IN_MEMORY);
    let io = |source| FrameError::Io {
        file: file.to_path_buf(),
        source,
    };
    let fit = format!(
        "scale='min({MODEL_EDGE},iw)':'min({MODEL_EDGE},ih)':force_original_aspect_ratio=decrease"
    );
    let mut child = tools
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-i", "-", "-frames:v", "1"])
        .args(["-vf", &fit, "-c:v", "mjpeg", "-q:v", QUALITY])
        .args(["-f", "image2pipe", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(io)?;
    let mut stdin = child
        .stdin
        .take()
        .expect("stdin was piped when the process was spawned");
    // Fed from its own thread: ffmpeg may write before it has read it all,
    // and a full stdout pipe would then wait on a stdin nobody drains.
    let (output, fed) = std::thread::scope(|scope| {
        let fed = scope.spawn(move || stdin.write_all(picture));
        let output = child.wait_with_output();
        let fed = fed.join().expect("writing to ffmpeg does not panic");
        (output, fed)
    });
    let output = output.map_err(io)?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(FrameError::Ffmpeg {
            file: file.to_path_buf(),
            message: stderr_of(&output.stderr),
        });
    }
    fed.map_err(io)?;
    Ok(output.stdout)
}
