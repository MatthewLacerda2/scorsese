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

use crate::error::{RenderError, Stage};
use crate::pipe::Process;
use crate::probe::Ffprobe;
use crate::tools::Tools;

/// The rate speech is sent at: what Whisper hears.
pub const SPEECH_RATE: u32 = 16_000;

/// `file`'s sound as a WAV: one channel, [`SPEECH_RATE`], sixteen-bit.
/// [`RenderError::NoSound`] for a file with none.
pub fn speech(tools: &Tools, file: &Path) -> Result<Vec<u8>, RenderError> {
    let subject = file.display().to_string();
    let sounds = Ffprobe::new(tools.clone())
        .probe(file)
        .ok()
        .and_then(|media| media.audio_channels)
        .is_some();
    if !sounds {
        return Err(RenderError::NoSound { file: subject });
    }
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error"])
        .arg("-i")
        .arg(file)
        .args(["-vn", "-ac", "1", "-ar", &SPEECH_RATE.to_string()])
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
    let child = Process::new(child);
    let mut wav = Vec::new();
    stdout
        .read_to_end(&mut wav)
        .map_err(|source| RenderError::Pipe {
            stage: Stage::Measure,
            source,
        })?;
    child.finish(Stage::Measure, &subject)?;
    Ok(wav)
}
