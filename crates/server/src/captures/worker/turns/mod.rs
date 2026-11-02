//! Whose page is captured next (#1634): one page a turn, and turns by owner.
//!
//! The launcher takes one capture at a time (#773), so its slot is shared by
//! every account. It used to take the oldest job and serve every page it
//! asked for before looking again, so one render with many pages, or several
//! renders in a row from one account, kept everybody else's pages — and so
//! their renders and stills — waiting. Now **a turn is one page**, or one
//! part of a long one: after each capture the worker picks again, and the
//! turn goes to the account whose last turn was longest ago (one never served first), the oldest of its jobs
//! first. A still (`still-<call>`) competes the same way. A job is answered
//! once its last page is captured; one withdrawn while it waits, or part way,
//! is dropped and never answered.
//!
//! **A turn draws at most [`crate::captures::TURN`] frames** (#1708): a longer
//! page is captured a part a turn (`Asked::parts`), its owner's later turns
//! taking the next part, and its last joins them into one capture.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::super::launch::{self, Staged};
use super::super::{ANSWER, ASK, Answer, Ask, PROJECT, publish, read};
use super::{Isolation, Part, Worker};

/// What the worker is part way through: each job it has started, and when
/// each owner last had a turn. A turn is one page, and goes to the owner whose
/// last turn was longest ago (never served first), its oldest job first.
#[derive(Debug, Default)]
pub struct Turns {
    started: HashMap<PathBuf, Started>,
    last: HashMap<String, u64>,
    taken: u64,
}

/// A job some of whose pages are captured.
#[derive(Debug)]
struct Started {
    ask: Ask,
    staged: Option<Staged>,
    answer: Answer,
    /// Which part of the page it is on (`Asked::parts`, #1708): the next to
    /// capture of the page after the last answered.
    part: usize,
}

/// A job waiting for its answer, in the order it was asked.
#[derive(Debug)]
pub(super) struct Waiting {
    /// When its `ask.json` was written, then its number.
    pub(super) order: (SystemTime, i64),
    /// Its folder.
    pub(super) path: PathBuf,
}

impl Worker {
    /// Every job waiting for an answer: a render's (`job-<n>`) or a still's
    /// (`still-<call>`, #1434), oldest first.
    pub(super) fn waiting(&self) -> Vec<Waiting> {
        let Ok(entries) = std::fs::read_dir(self.spool.jobs()) else {
            return Vec::new();
        };
        let mut waiting: Vec<Waiting> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                // A job is a folder of its own, never a link to one.
                if !entry.file_type().ok()?.is_dir() {
                    return None;
                }
                let path = entry.path();
                let name = path.file_name()?.to_str()?;
                let number: i64 = name
                    .strip_prefix("job-")
                    .or_else(|| name.strip_prefix("still-"))?
                    .parse()
                    .ok()?;
                let ask = path.join(ASK);
                let ready = ask.is_file() && !path.join(ANSWER).exists();
                let asked = std::fs::metadata(&ask).and_then(|meta| meta.modified());
                ready.then_some(Waiting {
                    order: (asked.ok()?, number),
                    path,
                })
            })
            .collect();
        waiting.sort_by_key(|job| job.order);
        waiting
    }

    /// Captures one page, the fairest one ([`Turns`] has the rule); whether there was one
    /// to capture.
    pub fn take_turn(&self, turns: &mut Turns) -> bool {
        self.forget_withdrawn(turns);
        let waiting = self.waiting();
        // A job's position is its owner's last turn, never-served first, and
        // then its own age: the order `waiting` is already in.
        let fairest = waiting
            .iter()
            .enumerate()
            .min_by_key(|(age, job)| {
                let owner = self.owner(&job.path);
                (turns.last.get(&owner).map_or(0, |turn| turn + 1), *age)
            })
            .map(|(_, job)| job.path.clone());
        let Some(job) = fairest else {
            return false;
        };
        let owner = self.owner(&job);
        turns.taken += 1;
        turns.last.insert(owner, turns.taken);
        self.turn_of(&job, turns);
        true
    }

    /// Captures every page the job at `job` asks for, turn after turn, and
    /// answers it — for a caller serving one job alone.
    pub fn serve(&self, job: &Path) {
        let mut turns = Turns::default();
        while self.turn_of(job, &mut turns) {}
    }

    /// Captures the next page of the job at `job`, starting it if it is new
    /// and answering it once that was its last; whether it has more to go.
    fn turn_of(&self, job: &Path, turns: &mut Turns) -> bool {
        if !job.join(ASK).is_file() {
            self.forget(turns, job);
            return false;
        }
        if !turns.started.contains_key(job) {
            match self.start(job) {
                Some(started) => turns.started.insert(job.to_path_buf(), started),
                None => return false,
            };
        }
        let started = turns.started.get_mut(job).expect("started above");
        let index = started.answer.failed.len();
        if let Some(asked) = started.ask.requests.get(index) {
            // A turn is one part of a page (#1708): a page is answered once
            // its last part is in, or as soon as one fails.
            let done = match asked.parts() {
                Ok(parts) => {
                    let part = Part {
                        frames: parts.get(started.part).cloned().unwrap_or_default(),
                        first: started.part == 0,
                        last: started.part + 1 >= parts.len(),
                    };
                    let captured = self.capture(job, index, asked, &part, started.staged.as_ref());
                    (captured.is_err() || part.last).then_some(captured.err())
                }
                Err(why) => Some(Some(why)),
            };
            match done {
                Some(failed) => {
                    started.answer.failed.push(failed);
                    started.part = 0;
                }
                None => started.part += 1,
            }
        }
        if started.answer.failed.len() < started.ask.requests.len() {
            return true;
        }
        let answer = std::mem::take(&mut started.answer);
        self.forget(turns, job);
        if job.join(ASK).is_file() {
            let _ = publish(&job.join(ANSWER), &answer);
        }
        false
    }

    /// The job at `job` read and staged, ready for its first turn — or
    /// answered at once, and `None`, when it cannot be.
    fn start(&self, job: &Path) -> Option<Started> {
        let Some(ask) = read::<Ask>(&job.join(ASK)) else {
            let _ = publish(&job.join(ANSWER), &Answer::default());
            return None;
        };
        let staged = match &self.isolation {
            Isolation::Container(launch) => match launch.stage(job, &ask) {
                Ok(staged) => Some(staged),
                Err(why) => {
                    let failed = vec![Some(why); ask.requests.len()];
                    let _ = publish(&job.join(ANSWER), &Answer { failed });
                    return None;
                }
            },
            Isolation::Process { .. } => None,
        };
        Some(Started {
            ask,
            staged,
            answer: Answer::default(),
            part: 0,
        })
    }

    /// Drops every started job its server withdrew — its `ask.json` gone.
    fn forget_withdrawn(&self, turns: &mut Turns) {
        let gone: Vec<PathBuf> = turns
            .started
            .keys()
            .filter(|job| !job.join(ASK).is_file())
            .cloned()
            .collect();
        for job in gone {
            self.forget(turns, &job);
        }
    }

    /// Drops the job at `job`, and what was staged for it.
    fn forget(&self, turns: &mut Turns, job: &Path) {
        let Some(started) = turns.started.remove(job) else {
            return;
        };
        if let (Isolation::Container(launch), Some(staged)) = (&self.isolation, &started.staged) {
            launch.unstage(staged);
        }
    }

    /// Whose the job at `job` is: the user whose page cache it captures
    /// into, or nobody's — one turn shared by every job that says no owner.
    fn owner(&self, job: &Path) -> String {
        launch::owner(self.spool.root(), &job.join(PROJECT)).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
