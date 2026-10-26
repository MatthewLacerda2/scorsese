//! A recording's speech, as a speech-to-text server is sent it (#1549).
//!
//! Whisper hears sixteen kilohertz mono, and resamples whatever it is sent to
//! that. Sending it that already makes a film's hour of sound 115 MB of WAV
//! rather than the film, and keeps every container ffmpeg can open readable
//! by a server that knows none of them. ffmpeg decodes; the bytes go out
//! whole.

use std::io::Read;
use std::path::Path;
use std::process::Stdio;

use scorsese_core::probe::ProbeMedia;

use crate::cancel::Cancel;
use crate::error::{RenderError, Stage};
use crate::pipe::{ALIGN_TO_FILE_CLOCK, Process};
use crate::probe::Ffprobe;
use crate::tools::Tools;

/// The rate speech is sent at: what Whisper hears.
pub const SPEECH_RATE: u32 = 16_000;

/// How much is read from ffmpeg at a time, between looks at the cancel.
const CHUNK: usize = 64 * 1024;

/// `file`'s sound as a WAV: one channel, [`SPEECH_RATE`], sixteen-bit, on the
/// file's own clock (`ALIGN_TO_FILE_CLOCK`), as the render mixes it — sound
/// that starts after the picture starts after silence, so a word's time is
/// where a render plays it.
///
/// Refused, before any of it is decoded, as [`RenderError::Unprobed`] for a
/// file ffprobe cannot read, [`RenderError::NoSound`] for a file with none and [`RenderError::TooLong`] for one its probe says lasts
/// past `longest` seconds; whatever no probe measured is cut there. Stopped,
/// ffmpeg killed, as [`RenderError::Stopped`] once `cancel` trips.
pub fn speech(
    tools: &Tools,
    file: &Path,
    longest: f64,
    cancel: &Cancel,
) -> Result<Vec<u8>, RenderError> {
    let subject = file.display().to_string();
    let probed =
        Ffprobe::new(tools.clone())
            .probe(file)
            .map_err(|error| RenderError::Unprobed {
                file: subject.clone(),
                why: error.message,
            })?;
    if probed.audio_channels.is_none() {
        return Err(RenderError::NoSound { file: subject });
    }
    if let Some(seconds) = probed.duration_seconds.filter(|&seconds| seconds > longest) {
        return Err(RenderError::TooLong {
            file: subject,
            seconds,
            longest,
        });
    }
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error"])
        .arg("-i")
        .arg(file)
        .args(["-t", &format!("{longest:.3}")])
        .args(["-vn", "-af", ALIGN_TO_FILE_CLOCK])
        .args(["-ac", "1", "-ar", &SPEECH_RATE.to_string()])
        .args(["-c:a", "pcm_s16le", "-f", "wav", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|source| RenderError::Spawn {
        stage: Stage::Measure,
        source,
    })?;
    let mut stdout = child
        .stdout
        .take()
        .expect("stdout was piped when the process was spawned");
    // Dropped on a stop, which kills it.
    let child = Process::new(child);
    let mut wav = Vec::new();
    let mut chunk = vec![0; CHUNK];
    loop {
        if cancel.is_cancelled() {
            return Err(RenderError::Stopped { file: subject });
        }
        let read = stdout
            .read(&mut chunk)
            .map_err(|source| RenderError::Pipe {
                stage: Stage::Measure,
                source,
            })?;
        if read == 0 {
            break;
        }
        wav.extend_from_slice(&chunk[..read]);
    }
    child.finish(Stage::Measure, &subject)?;
    Ok(wav)
}
