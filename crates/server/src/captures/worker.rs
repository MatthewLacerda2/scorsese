//! The loop that answers the spool, and the `scorsese-server capture-worker`
//! command that runs it without containers.
//!
//! It watches the spool ([`super`]) for a job whose `ask.json` is there and
//! whose `answer.json` is not, oldest first, and captures each page it asks
//! for — one at a time, each in a child process of its own process group, so
//! that at the deadline it and everything it started are killed together.
//! Then it answers, and looks again. It never touches a database or a network.
//!
//! **How each capture is walled in** is its [`Isolation`]. The hosted service
//! runs the loop as `capture-launcher`, where each capture is a container of
//! its own ([`super::launch`], #852). `capture-worker` runs each as a bare
//! `capture-one` child of this binary: for a machine without Docker, and for
//! the tests, where the walls are not what is being tested.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use super::launch::{self, Launch};
use super::{ANSWER, ASK, Answer, Ask, Asked, PROJECT, Spool, now, publish, read};

/// `scorsese-server capture-worker`
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The spool the server writes jobs into: `captures/` under its
    /// `SCORSESE_CACHE`, mounted at the same path here.
    #[arg(long)]
    pub spool: PathBuf,
    /// Run the browser without its sandbox. For CI and development machines,
    /// which run as root, where Chromium refuses one. The launcher's
    /// containers never pass it: there the sandbox is on, with no opt-out
    /// (#594).
    #[arg(long)]
    pub no_sandbox: bool,
}

/// How often an idle worker looks for a job, and a busy one at its child.
const TICK: Duration = Duration::from_millis(250);

/// What each capture runs in.
#[derive(Debug, Clone)]
pub enum Isolation {
    /// A `capture-one` child of `program`, with the browser's sandbox on or
    /// off ([`Args::no_sandbox`]) and no other wall.
    Process {
        /// The binary to run `capture-one` with — this one, outside a test.
        program: PathBuf,
        /// Whether the browser runs with its sandbox.
        sandbox: bool,
    },
    /// A container of its own, mounting only its job ([`Launch`]).
    Container(Launch),
}

/// The worker: where it reads, what it runs, and how long a capture may take.
#[derive(Debug, Clone)]
pub struct Worker {
    /// The spool.
    pub spool: Spool,
    /// What each capture runs in.
    pub isolation: Isolation,
    /// How long a capture of so many frames may take ([`super::deadline`]).
    pub deadline: fn(u64) -> Duration,
}

impl Worker {
    /// From the command line, running this binary for each capture.
    pub fn from_args(args: &Args) -> std::io::Result<Self> {
        Ok(Self {
            spool: Spool::new(&args.spool),
            isolation: Isolation::Process {
                program: std::env::current_exe()?,
                sandbox: !args.no_sandbox,
            },
            deadline: super::deadline,
        })
    }

    /// The launcher: each capture a container of its own.
    pub fn launching(launch: Launch) -> Self {
        Self {
            spool: launch.spool(),
            isolation: Isolation::Container(launch),
            deadline: super::deadline,
        }
    }

    /// Works until the process is stopped.
    pub fn run(&self) -> ! {
        loop {
            self.beat();
            match self.next() {
                Some(job) => self.serve(&job),
                None => std::thread::sleep(TICK),
            }
        }
    }

    /// The oldest job waiting for an answer, if one is.
    pub fn next(&self) -> Option<PathBuf> {
        let waiting = std::fs::read_dir(self.spool.jobs())
            .ok()?
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let number: i64 = path
                    .file_name()?
                    .to_str()?
                    .strip_prefix("job-")?
                    .parse()
                    .ok()?;
                let ready = path.join(ASK).is_file() && !path.join(ANSWER).exists();
                ready.then_some((number, path))
            });
        waiting
            .min_by_key(|(number, _)| *number)
            .map(|(_, path)| path)
    }

    /// Captures what the job at `job` asks for, and answers it. A job its
    /// server withdrew while it ran — the folder gone — gets no answer.
    pub fn serve(&self, job: &Path) {
        let Some(ask) = read::<Ask>(&job.join(ASK)) else {
            let _ = publish(&job.join(ANSWER), &Answer::default());
            return;
        };
        let mut answer = Answer::default();
        for (index, asked) in ask.requests.iter().enumerate() {
            if !job.join(ASK).is_file() {
                return;
            }
            answer.failed.push(self.capture(job, index, asked).err());
        }
        if job.is_dir() {
            let _ = publish(&job.join(ANSWER), &answer);
        }
    }

    /// One capture, in a child the deadline can kill; why it failed if it did.
    fn capture(&self, job: &Path, index: usize, asked: &Asked) -> Result<(), String> {
        let frames = asked.request()?.frames();
        let limit = (self.deadline)(frames);
        let mut command = self.command(job, index)?;
        let mut child = command
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("the page renderer would not start: {error}"))?;
        let mut stderr = child.stderr.take().expect("stderr was piped");
        let said = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        let group = child.id();
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if started.elapsed() < limit => {
                    self.beat();
                    std::thread::sleep(TICK);
                }
                _ => break None,
            }
        };
        // Whatever the child left behind goes with it: a browser that outlived
        // its capture is a browser still running somebody's page. A
        // container is not the client's child, so it is stopped by name.
        if let Isolation::Container(_) = self.isolation {
            launch::kill(&Launch::name(job, index));
        }
        kill_group(group);
        let _ = child.wait();
        let said = said.join().unwrap_or_default();
        match status {
            Some(status) if status.success() => Ok(()),
            Some(status) => Err(failure(&said, status)),
            None => Err(format!(
                "the page was stopped after {} seconds, the most a capture of {frames} frames \
                 may take; a page whose clock never settles, or that does a lot of work each \
                 frame, runs past it",
                limit.as_secs()
            )),
        }
    }

    /// What runs capture `index` of the job at `job`.
    fn command(&self, job: &Path, index: usize) -> Result<Command, String> {
        match &self.isolation {
            Isolation::Container(launch) => launch.command(job, index),
            Isolation::Process { program, sandbox } => {
                let mut command = Command::new(program);
                command
                    .arg("capture-one")
                    .arg("--project")
                    .arg(job.join(PROJECT))
                    .arg("--ask")
                    .arg(job.join(ASK))
                    .arg("--index")
                    .arg(index.to_string());
                if !sandbox {
                    command.arg("--no-sandbox");
                }
                Ok(command)
            }
        }
    }

    /// Says the worker is running, for the server to see ([`super::dispatch`]).
    fn beat(&self) {
        let _ = std::fs::write(self.spool.alive(), now().to_string());
    }
}

/// Why a `capture-one` that exited said it failed: its last line, which is
/// the capture's own error.
fn failure(said: &str, status: ExitStatus) -> String {
    said.lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map_or_else(
            || format!("the page renderer stopped ({status})"),
            |line| line.trim_start_matches("scorsese-server: ").to_owned(),
        )
}

/// Kills every process in the group `group` leads. Through a shell's `kill`,
/// which takes a negative pid for a group: the standard library signals one
/// process, and this workspace forbids the `unsafe` a raw `kill(2)` takes.
fn kill_group(group: u32) {
    let _ = Command::new("/bin/sh")
        .arg("-c")
        .arg(r#"kill -KILL "-$0" 2>/dev/null"#)
        .arg(group.to_string())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_is_the_last_thing_the_capture_said() {
        let status = Command::new("/bin/sh")
            .args(["-c", "exit 1"])
            .status()
            .unwrap();
        assert_eq!(
            failure(
                "noise\nscorsese-server: the page would not load: boom\n\n",
                status
            ),
            "the page would not load: boom"
        );
        assert!(failure("", status).starts_with("the page renderer stopped"));
    }
}
