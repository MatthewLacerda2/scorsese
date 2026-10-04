//! The renders a session has started, and how far each has got (#700).
//!
//! A render runs for minutes or hours, and the stdio server answers one call
//! at a time. While `render` held the serving thread until the file was
//! written, nothing could ask how far it had got — the assistant could only
//! wait, and the person waiting with it learned nothing. So a render is a
//! **job**: it runs on a thread of its own, publishing to a
//! [`scorsese_render::Progress`], and `jobs` reads that readout whenever it is
//! asked. The web's queue (#541) works the same way for its own reasons, and
//! the tool names and the line a job is described in are kept the same, so an
//! assistant's habits carry from one to the other.
//!
//! ## The one thing a session holds
//!
//! Every tool is otherwise stateless. These jobs are not, and cannot be: a
//! render is a running thread, and the session is what owns it. They belong to
//! one `serve`, and they end with it — see [`Renders`]'s `Drop`.

mod watch;
mod words;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use scorsese_render::{Cancel, Progress, Reading};

pub(crate) use watch::{Report, Say, watch};
pub(crate) use words::{doing, line};

/// What a render ends with: what it says about the file it wrote, or why
/// there is no file.
pub(crate) type Outcome = Result<String, String>;

/// One render, running or finished.
pub(crate) struct Job {
    /// How a client names it: `1` for a session's first render, and up.
    pub(crate) id: u64,
    /// The project it renders, as found on disk.
    project: PathBuf,
    /// Where it writes, as the caller said it — the words the answer uses.
    pub(crate) out: String,
    /// Where it writes, resolved: what two renders must not share.
    path: PathBuf,
    progress: Progress,
    cancel: Cancel,
    outcome: Mutex<Option<Outcome>>,
    ended: Condvar,
    thread: Mutex<Option<JoinHandle<()>>>,
}

/// Where a job is, in a form its wording can match on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum State {
    /// Still going, this far along.
    Running(Reading),
    /// Finished; what it says about the file.
    Done(String),
    /// Stopped by `job_cancel` or a cancelled call; how far it got.
    Cancelled(String),
    /// Refused or broke partway; why.
    Failed(String),
}

impl Job {
    /// Where it is now.
    pub(crate) fn state(&self) -> State {
        match lock(&self.outcome).clone() {
            None => State::Running(self.progress.read()),
            Some(Ok(said)) => State::Done(said),
            Some(Err(why)) if self.cancel.is_cancelled() => State::Cancelled(why),
            Some(Err(why)) => State::Failed(why),
        }
    }

    /// Asks it to stop. It does within a frame, and removes the file it began.
    pub(crate) fn cancel(&self) {
        self.cancel.cancel();
    }

    /// How far it has got, read this moment.
    pub(crate) fn reading(&self) -> Reading {
        self.progress.read()
    }

    /// Its outcome, waiting up to `tick` for one; `None` while it runs on.
    pub(crate) fn ended_within(&self, tick: Duration) -> Option<Outcome> {
        let held = lock(&self.outcome);
        let (held, _) = self
            .ended
            .wait_timeout_while(held, tick, |outcome| outcome.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        held.clone()
    }

    /// Whether it renders `project`.
    pub(crate) fn renders(&self, project: &Path) -> bool {
        self.project == found(project)
    }

    fn running(&self) -> bool {
        lock(&self.outcome).is_none()
    }

    fn end(&self, outcome: Outcome) {
        *lock(&self.outcome) = Some(outcome);
        self.ended.notify_all();
    }
}

/// What a job is asked to do: render, publishing to the readout, under the
/// cancel — and say what it made.
pub(crate) type Work = Box<dyn FnOnce(Progress, Cancel) -> Outcome + Send>;

/// Every render a session has started, oldest first.
#[derive(Default)]
pub(crate) struct Renders {
    jobs: Mutex<Vec<Arc<Job>>>,
}

impl Renders {
    /// Starts `work` on a thread of its own, writing `path`, under `cancel`.
    ///
    /// Refused while another of this session's renders is still writing the
    /// same file: two encoders on one path make one file neither of them
    /// wrote.
    pub(crate) fn start(
        &self,
        project: &Path,
        (out, path): (&str, PathBuf),
        cancel: Cancel,
        work: Work,
    ) -> Result<Arc<Job>, String> {
        let mut jobs = lock(&self.jobs);
        if let Some(busy) = jobs.iter().find(|job| job.path == path && job.running()) {
            return Err(format!(
                "job {} is still writing {out} — wait for it with jobs, or stop it \
                 with job_cancel, before rendering there again",
                busy.id
            ));
        }
        let job = Arc::new(Job {
            id: jobs.len() as u64 + 1,
            project: found(project),
            out: out.to_owned(),
            path,
            progress: Progress::new(),
            cancel,
            outcome: Mutex::new(None),
            ended: Condvar::new(),
            thread: Mutex::new(None),
        });
        let running = Arc::clone(&job);
        let thread = std::thread::spawn(move || {
            let outcome = work(running.progress.clone(), running.cancel.clone());
            running.end(outcome);
        });
        *lock(&job.thread) = Some(thread);
        jobs.push(Arc::clone(&job));
        Ok(job)
    }

    /// The job called `id`, if this session started one.
    pub(crate) fn get(&self, id: u64) -> Option<Arc<Job>> {
        lock(&self.jobs).iter().find(|job| job.id == id).cloned()
    }

    /// This session's renders of `project`, newest first.
    pub(crate) fn of(&self, project: &Path) -> Vec<Arc<Job>> {
        let jobs = lock(&self.jobs);
        jobs.iter()
            .rev()
            .filter(|job| job.renders(project))
            .cloned()
            .collect()
    }
}

/// The session is over — the client closed the pipe, or the server is
/// exiting — so every render still running is stopped, and waited for.
///
/// Stopped rather than left to finish, because nobody is left to learn where
/// the file went: the next session has never heard of this job, and a render
/// that carries on spends the person's machine on a file no conversation knows
/// exists. Waited for, so each one removes the file it had begun before the
/// process ends, rather than leaving a truncated `.mp4` that looks finished.
impl Drop for Renders {
    fn drop(&mut self) {
        let jobs = std::mem::take(&mut *lock(&self.jobs));
        for job in &jobs {
            job.cancel();
        }
        for job in jobs {
            if let Some(thread) = lock(&job.thread).take() {
                let _ = thread.join();
            }
        }
    }
}

/// The project directory as the filesystem names it, so `x.scor` and
/// `./x.scor` are one project; as given when it cannot be resolved.
fn found(project: &Path) -> PathBuf {
    std::fs::canonicalize(project).unwrap_or_else(|_| project.to_owned())
}

/// A lock no code under it can poison — nothing held under one panics — used
/// as it stands if one ever were.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
