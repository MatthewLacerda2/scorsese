//! Which projects a call is changing, so that two calls never change one at
//! once (#1063).
//!
//! Every tool that edits a project loads the document, changes it and saves
//! the whole thing back. Two of those overlapping on one project is a race
//! `Project::save`'s baseline turns from a silent loss into a refusal — but
//! only *a* refusal, and only to within the moment between its check and its
//! write (`scorsese_core::Baseline` says how far that goes). Inside one
//! session the calls are this process's own, so they can simply take turns.
//!
//! A turn is the **whole call**, not the save: the second writer's document
//! has to be read after the first one's landed, or it is built on what that
//! one replaced. That is safe here where it is not in `core` — a call that
//! dies releases its hold as it unwinds, and the session that holds it ends
//! with the process.
//!
//! **Waiting has a limit.** A burst of edits takes milliseconds each and
//! queues invisibly. A call behind a `generate` would wait minutes saying
//! nothing, which is the hang this exists to remove — so after [`PATIENCE`]
//! it is refused instead, naming the call it is behind.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// How long a call waits for another to finish changing the same project.
///
/// Far longer than any burst of ordinary edits, which take milliseconds each,
/// or an import probing a folder of files. Far shorter than the two minutes
/// after which Claude Code gives up waiting on a call and moves it to the
/// background — so a client hears why, rather than that it is still running.
pub(super) const PATIENCE: Duration = Duration::from_secs(30);

/// The projects being changed, each with the tool changing it.
#[derive(Default)]
pub(super) struct Holds {
    held: Mutex<HashMap<PathBuf, &'static str>>,
    freed: Condvar,
}

/// A project held by one call, until this is dropped.
pub(super) struct Held<'a> {
    holds: &'a Holds,
    project: PathBuf,
}

impl Holds {
    /// Holds `project` for `tool`, waiting up to [`PATIENCE`] for whoever is
    /// changing it now.
    pub(super) fn take(&self, project: &str, tool: &'static str) -> Result<Held<'_>, String> {
        self.take_within(project, tool, PATIENCE)
    }

    /// [`Holds::take`], with how long to wait said.
    pub(super) fn take_within(
        &self,
        project: &str,
        tool: &'static str,
        patience: Duration,
    ) -> Result<Held<'_>, String> {
        let key = key(project);
        let deadline = Instant::now() + patience;
        let mut held = self.lock();
        while let Some(&holder) = held.get(&key) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(format!(
                    "`{holder}` is still changing {project}, and this call waited \
                     {}s for it — nothing was changed. Calls that only read the \
                     project go ahead meanwhile; make this change again once \
                     `{holder}` has answered.",
                    patience.as_secs()
                ));
            }
            held = self
                .freed
                .wait_timeout(held, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        held.insert(key.clone(), tool);
        Ok(Held {
            holds: self,
            project: key,
        })
    }

    /// The table, through a poisoned lock if it comes to it: nothing under it
    /// can panic half-way through changing it.
    fn lock(&self) -> MutexGuard<'_, HashMap<PathBuf, &'static str>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        self.holds.lock().remove(&self.project);
        self.holds.freed.notify_all();
    }
}

/// One name per project however it was spelled — `a.scor`, `./a.scor` and its
/// absolute path are one project. A directory that does not exist yet (a
/// `project_new`) is made absolute without resolving links, which is all
/// there is to resolve.
fn key(project: &str) -> PathBuf {
    let path = Path::new(project);
    path.canonicalize()
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf())
}
