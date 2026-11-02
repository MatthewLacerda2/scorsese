//! A child whose work is the file it writes, waited for unless a cancel
//! comes first.

use std::io::Read;
use std::process::{Command, Output};

use super::process::POLL;
use crate::cancel::Cancel;

/// `command`'s [`Command::output`], unless `cancel` is tripped first: then the
/// child is killed and reaped, and the answer is `None`. Its stdout is not
/// kept: for a child whose work is the file it writes (a proxy, a thumbnail),
/// started with its stdout and stdin already set.
pub(crate) fn output_unless(
    command: &mut Command,
    cancel: &Cancel,
) -> std::io::Result<Option<Output>> {
    let mut child = command.stderr(std::process::Stdio::piped()).spawn()?;
    let said = child.stderr.take().map(|mut stderr| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.read_to_end(&mut bytes);
            bytes
        })
    });
    let status = loop {
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(POLL),
            Err(error) => {
                // Not left running where nothing can stop it.
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
    };
    let stderr = said.and_then(|said| said.join().ok()).unwrap_or_default();
    Ok(Some(Output {
        status,
        stdout: Vec::new(),
        stderr,
    }))
}
