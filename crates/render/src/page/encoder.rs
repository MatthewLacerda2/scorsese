//! The ffmpeg a capture's pictures are piped into as they are drawn.

use std::io::Write;
use std::path::Path;
use std::process::{ChildStdin, Stdio};

use super::PageError;
use super::request::Request;
use crate::error::Stage;
use crate::tools::Tools;

/// The ffmpeg turning a stream of PNGs into one lossless file with alpha.
///
/// FFV1 in Matroska, as `bgra`: lossless, carries alpha, and decoded by the
/// same path as any other video with an alpha channel. #606's PNGs ran to
/// ~1.5 MB a frame; this keeps the same pixels for a fraction of that.
pub(crate) struct Encoder {
    child: crate::pipe::Process,
    stdin: ChildStdin,
    subject: String,
}

impl Encoder {
    pub(crate) fn start(tools: &Tools, request: &Request, out: &Path) -> Result<Self, PageError> {
        let rate = format!("{}/{}", request.fps.num(), request.fps.den());
        let mut child = tools
            .ffmpeg()
            .args([
                "-nostdin",
                "-v",
                "error",
                "-y",
                "-f",
                "image2pipe",
                "-framerate",
                &rate,
            ])
            .args([
                "-c:v", "png", "-i", "-", "-c:v", "ffv1", "-pix_fmt", "bgra", "-f", "matroska",
            ])
            .arg(out)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| crate::RenderError::Spawn {
                stage: Stage::Encode,
                source,
            })?;
        let stdin = child.stdin.take().expect("stdin was piped");
        Ok(Self {
            child: crate::pipe::Process::new(child),
            stdin,
            subject: out.display().to_string(),
        })
    }

    pub(crate) fn write(&mut self, png: &[u8]) -> Result<(), PageError> {
        self.stdin.write_all(png).map_err(|source| {
            PageError::Render(crate::RenderError::Pipe {
                stage: Stage::Encode,
                source,
            })
        })
    }

    pub(crate) fn finish(self) -> Result<(), PageError> {
        drop(self.stdin);
        Ok(self.child.finish(Stage::Encode, &self.subject)?)
    }
}
