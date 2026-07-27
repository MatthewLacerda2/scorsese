//! Getting single frames in and out of files.
//!
//! References are PNGs, and they are written and read through ffmpeg like
//! everything else — no image library, and no exception to the rule that every
//! ffmpeg invocation goes through the command builder.
//!
//! PNG because it is lossless, because a 64×64 reference is a few hundred bytes
//! rather than a binary blob, and because a review page renders an image diff:
//! re-blessing a golden shows up as a picture that changed, which is the only
//! form in which a human can judge whether the change was legitimate.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use scorsese_render::{Frame, PIXEL_FORMAT, Resolution, Tools};

/// Decodes one frame of a video file by index.
pub fn extract(
    tools: &Tools,
    file: &Path,
    index: u64,
    resolution: Resolution,
) -> Result<Frame, FrameError> {
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(file)
        .args(["-vf", &format!("select=eq(n\\,{index})")])
        .args(["-frames:v", "1"]);
    raw_frame(command, file, resolution)
}

/// Reads a reference PNG.
pub fn read_reference(
    tools: &Tools,
    file: &Path,
    resolution: Resolution,
) -> Result<Frame, FrameError> {
    let mut command = tools.ffmpeg();
    command.args(["-nostdin", "-v", "error", "-i"]).arg(file);
    raw_frame(command, file, resolution)
}

/// Writes a frame out as a reference PNG, creating the directory if needed.
pub fn write_reference(tools: &Tools, file: &Path, frame: &Frame) -> Result<(), FrameError> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FrameError::Io {
            file: parent.to_path_buf(),
            source,
        })?;
    }
    let mut child = tools
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-y"])
        .args(["-f", "rawvideo", "-pix_fmt", PIXEL_FORMAT])
        .args(["-s", &frame.resolution().to_string()])
        .args(["-i", "-", "-frames:v", "1"])
        .arg(file)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| FrameError::Io {
            file: file.to_path_buf(),
            source,
        })?;

    let mut stdin = child
        .stdin
        .take()
        .expect("stdin was piped when the process was spawned");
    stdin
        .write_all(frame.bytes())
        .map_err(|source| FrameError::Io {
            file: file.to_path_buf(),
            source,
        })?;
    drop(stdin);

    let output = child.wait_with_output().map_err(|source| FrameError::Io {
        file: file.to_path_buf(),
        source,
    })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(FrameError::Ffmpeg {
            file: file.to_path_buf(),
            message: stderr_of(&output.stderr),
        })
    }
}

/// Runs a command expected to emit exactly one raw RGBA frame on stdout.
fn raw_frame(
    mut command: std::process::Command,
    file: &Path,
    resolution: Resolution,
) -> Result<Frame, FrameError> {
    let output = command
        .args(["-f", "rawvideo", "-pix_fmt", PIXEL_FORMAT, "-"])
        .stdin(Stdio::null())
        .output()
        .map_err(|source| FrameError::Io {
            file: file.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(FrameError::Ffmpeg {
            file: file.to_path_buf(),
            message: stderr_of(&output.stderr),
        });
    }

    let mut frame = Frame::black(resolution);
    let wanted = frame.byte_count();
    if output.stdout.len() != wanted {
        return Err(FrameError::WrongSize {
            file: file.to_path_buf(),
            found: output.stdout.len(),
            wanted,
            resolution,
        });
    }
    frame.bytes_mut().copy_from_slice(&output.stdout);
    Ok(frame)
}

fn stderr_of(stderr: &[u8]) -> String {
    let message = String::from_utf8_lossy(stderr);
    let message = message.trim();
    if message.is_empty() {
        "ffmpeg reported no detail".to_owned()
    } else {
        message.to_owned()
    }
}

/// Why a frame could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    /// ffmpeg could not be spawned, or its stdin could not be fed.
    #[error("{}: {source}", file.display())]
    Io {
        /// The frame file being read or written when this happened.
        file: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
    /// ffmpeg ran and refused the file — an unreadable PNG, or a codec it
    /// cannot open.
    #[error("ffmpeg failed on {}: {message}", file.display())]
    Ffmpeg {
        /// The file ffmpeg was pointed at.
        file: PathBuf,
        /// ffmpeg's own stderr, trimmed.
        message: String,
    },
    /// ffmpeg succeeded but handed back the wrong number of bytes, which means
    /// the frame is not what was asked for even though nothing errored.
    #[error(
        "{} decoded to {found} bytes, but one {resolution} frame is {wanted} — \
         either the frame is not there or the reference is the wrong size",
        file.display()
    )]
    WrongSize {
        /// The video or reference PNG that was decoded.
        file: PathBuf,
        /// Bytes ffmpeg actually wrote — zero when the frame index is past the
        /// end of the file.
        found: usize,
        /// Bytes one frame at `resolution` occupies.
        wanted: usize,
        /// The size the caller expected, from the fixture's render settings.
        resolution: Resolution,
    },
}
