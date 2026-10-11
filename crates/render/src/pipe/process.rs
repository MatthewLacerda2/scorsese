//! The ffmpeg children a render spawns, and what they say on the way.

use std::io::Read;
use std::process::Child;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::error::{RenderError, Stage};

/// How much of what ffmpeg says on stderr is kept for an error: the tail,
/// where the reason it gave up is. A damaged source can print hundreds of
/// kilobytes of `error while decoding MB` before that (#1084), and none of it
/// is worth holding.
const KEPT: usize = 16 * 1024;

/// An ffmpeg child that is reaped however the render that spawned it ends, and
/// whose stderr is read as it is written.
///
/// Dropping a [`Child`] neither kills nor waits for it, so a render that
/// returned early — a failure, a cancel (#647) — used to leave its decoders and
/// its encoder running, or as zombies under a server that lives for hours. A
/// process that is not [`Process::finish`]ed or [`Process::stop`]ped on purpose
/// is killed and waited for when this drops.
///
/// **Stderr is drained on a thread from the moment it starts.** A pipe holds
/// about 64 KB; read only once the process ended, a chatty ffmpeg filled it,
/// blocked on the write, stopped producing stdout, and the render waiting on
/// that stdout waited for good — where no cancel could reach it (#1084).
pub(crate) struct Process {
    child: Option<Child>,
    said: Option<JoinHandle<Said>>,
}

impl Process {
    /// Takes charge of a freshly spawned child, and starts reading its stderr
    /// when it was piped.
    pub(crate) fn new(mut child: Child) -> Self {
        let said = child
            .stderr
            .take()
            .map(|stderr| std::thread::spawn(move || Said::read(stderr, KEPT)));
        Self {
            child: Some(child),
            said,
        }
    }

    /// Waits for it to end on its own, and turns a non-zero exit into an error
    /// carrying the end of whatever it said on stderr.
    pub(crate) fn finish(mut self, stage: Stage, subject: &str) -> Result<(), RenderError> {
        let mut child = self.child.take().expect("a process is only finished once");
        let status = child
            .wait()
            .map_err(|source| RenderError::Pipe { stage, source })?;
        let said = self
            .said
            .take()
            .and_then(|said| said.join().ok())
            .unwrap_or_default();
        if status.success() {
            return Ok(());
        }
        let message = said.message();
        Err(RenderError::Ffmpeg {
            stage,
            subject: subject.to_owned(),
            message: if message.is_empty() {
                format!("exited with {status}")
            } else {
                message
            },
        })
    }

    /// Gives it `grace` to exit by itself — its stdin already closed, which is
    /// how ffmpeg is asked to stop — and kills it when it has not.
    ///
    /// Patient first because a polite exit is ffmpeg's own; the kill is for
    /// one that is wedged, as the encoder in #647 was, ignoring everything
    /// short of `SIGKILL`.
    pub(crate) fn stop(mut self, grace: Duration) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // The reading thread is left to end by itself: the pipe closes when
        // the process does, and nothing is waiting on what it kept.
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Waits for a freshly spawned ffmpeg, as [`Process::finish`] does — for a
/// process with nothing to read from it but its exit.
pub(crate) fn finish(child: Child, stage: Stage, subject: &str) -> Result<(), RenderError> {
    Process::new(child).finish(stage, subject)
}

/// The end of what a process said on stderr, and whether anything before it
/// was dropped.
#[derive(Debug, Default)]
struct Said {
    tail: Vec<u8>,
    cut: bool,
}

impl Said {
    /// Reads `stderr` to its end, keeping the last `kept` bytes of it.
    fn read(mut stderr: impl Read, kept: usize) -> Self {
        let mut tail = Vec::with_capacity(kept.min(4096));
        let mut chunk = [0_u8; 4096];
        let mut cut = false;
        loop {
            match stderr.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => tail.extend_from_slice(&chunk[..read]),
            }
            // Twice over before trimming, so the copy is paid once per `kept`
            // bytes read rather than once per read.
            if tail.len() > 2 * kept {
                tail.drain(..tail.len() - kept);
                cut = true;
            }
        }
        if tail.len() > kept {
            tail.drain(..tail.len() - kept);
            cut = true;
        }
        Self { tail, cut }
    }

    /// What it said, trimmed. A tail that was cut starts on a whole line, with
    /// an ellipsis saying there was more before it.
    fn message(&self) -> String {
        let mut bytes = self.tail.as_slice();
        if self.cut
            && let Some(newline) = bytes.iter().position(|&byte| byte == b'\n')
        {
            bytes = &bytes[newline + 1..];
        }
        let text = String::from_utf8_lossy(bytes);
        let text = text.trim();
        if self.cut && !text.is_empty() {
            format!("…\n{text}")
        } else {
            text.to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_fits_is_kept_whole() {
        let said = Said::read(&b"  Invalid data found\n"[..], 64);
        assert!(!said.cut);
        assert_eq!(said.message(), "Invalid data found");
    }

    #[test]
    fn a_flood_keeps_only_its_last_whole_lines() {
        let flood: Vec<u8> = (0..1000)
            .flat_map(|line| format!("error while decoding MB {line}\n").into_bytes())
            .chain(b"Conversion failed!\n".iter().copied())
            .collect();
        let said = Said::read(flood.as_slice(), 256);
        assert!(said.cut);
        assert!(said.tail.len() <= 256);
        let message = said.message();
        assert!(
            message.starts_with("…\nerror while decoding MB"),
            "{message}"
        );
        assert!(message.ends_with("MB 999\nConversion failed!"), "{message}");
    }

    /// The hang itself: a child that says far more than a pipe holds on
    /// stderr before it writes its stdout. Unread, it blocks on stderr and
    /// the read of stdout never ends; drained, both finish.
    #[cfg(unix)]
    #[test]
    fn a_child_flooding_stderr_still_reaches_its_stdout() {
        use std::process::{Command, Stdio};
        use std::sync::mpsc;

        let script = "head -c 1000000 /dev/zero | tr '\\0' x >&2; echo done; exit 3";
        let (sent, arrived) = mpsc::channel();
        std::thread::spawn(move || {
            let mut child = Command::new("/bin/sh")
                .args(["-c", script])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("a shell");
            let mut stdout = child.stdout.take().expect("piped");
            let process = Process::new(child);
            let mut said = String::new();
            stdout.read_to_string(&mut said).expect("stdout");
            let _ = sent.send((said, process.finish(Stage::Decode, "flood")));
        });

        let (said, finished) = arrived
            .recv_timeout(Duration::from_secs(30))
            .expect("a full stderr pipe must not stall the child");
        assert_eq!(said, "done\n");
        let Err(RenderError::Ffmpeg { message, .. }) = finished else {
            panic!("a non-zero exit is an error: {finished:?}");
        };
        assert!(message.len() <= KEPT + 8, "kept {} bytes", message.len());
    }

    #[test]
    fn nothing_said_is_an_empty_message() {
        assert_eq!(Said::read(&b""[..], 64).message(), "");
    }
}
