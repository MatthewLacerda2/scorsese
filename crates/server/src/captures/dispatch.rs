//! The server's side of the spool: a render job asks for its pages and waits.
//! A still asks the same way ([`super::still`]), through the helpers here.
//!
//! The job has laid its project out in its spool folder ([`Pages::folder`]).
//! This points that folder's `cache/` at the project's page cache, writes the
//! ask, and waits for the worker's answer — or for the job to be cancelled,
//! or for the worker to be plainly not running. The render then draws every
//! page from that cache and never starts a browser itself
//! ([`scorsese_render::Renderer::without_capturing`]).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use scorsese_core::CACHE_DIR;
use scorsese_render::Cancel;
use scorsese_render::page::{CHROME_ENV, Chrome, Wanted};

use super::{ANSWER, ASK, Answer, Ask, Asked, PROJECT, Spool, now, publish, read};

/// How stale the worker's last look may be before the server stops waiting
/// for it: it looks at least every second, busy or not.
const SILENCE: Duration = Duration::from_secs(30);

/// How often the server looks for the answer.
const TICK: Duration = Duration::from_millis(250);

/// Why no page was captured when the worker is not there to capture them.
pub(super) const NOT_RUNNING: &str =
    "the page renderer is not running on this server; the pages are drawn once it is";

/// One render job's pages: which job's folder, and where its project keeps
/// its captures ([`Spool::pages`]).
#[derive(Debug, Clone)]
pub struct Pages {
    /// The spool.
    pub spool: Spool,
    /// The job asking.
    pub job: i64,
    /// The project's page cache, which its laid-out `cache/` links to.
    pub cache: PathBuf,
}

/// What came of asking: the browser the captures are kept under, and why each
/// page that was not captured was not, by its path.
#[derive(Debug)]
pub struct Captured {
    /// The build the worker captures with — the same image, so the same one.
    /// `None` when the server has none, and no page was captured.
    pub chrome: Option<Chrome>,
    /// Why, for each page path that has no capture.
    pub failed: HashMap<String, String>,
}

impl Pages {
    /// The job's folder in the spool, removed when the job ends.
    pub fn job_folder(&self) -> PathBuf {
        self.spool.job(self.job)
    }

    /// Where to lay the project out, so the worker can see it.
    pub fn folder(&self) -> PathBuf {
        self.job_folder().join(PROJECT)
    }

    /// Captures `requests` for the project laid out at [`Pages::folder`], only
    /// the frames each asks for, and waits for them. `Err` only when `cancel`
    /// was tripped.
    pub fn capture(&self, requests: &[Wanted], cancel: &Cancel) -> Result<Captured, String> {
        let chrome = match browser() {
            Ok(chrome) => chrome,
            Err(why) => return Ok(none_captured(requests, &why, None)),
        };
        if let Err(error) = share_cache(&self.folder(), &self.cache) {
            let why = format!("the page cache could not be prepared: {error}");
            return Ok(none_captured(requests, &why, Some(chrome)));
        }
        let ask = Ask {
            requests: requests.iter().map(Asked::from).collect(),
        };
        if let Err(error) = publish(&self.job_folder().join(ASK), &ask) {
            let why = format!("the page renderer could not be asked: {error}");
            return Ok(none_captured(requests, &why, Some(chrome)));
        }
        let answer = match wait(&self.spool, &self.job_folder(), cancel, None)? {
            Waited::Answered(answer) => answer,
            Waited::Quiet | Waited::Gone | Waited::Late => {
                return Ok(none_captured(requests, NOT_RUNNING, Some(chrome)));
            }
        };
        let failed = requests
            .iter()
            .zip(answer.failed)
            .filter_map(|(wanted, why)| Some((wanted.request.page.clone(), why?)))
            .collect();
        Ok(Captured {
            chrome: Some(chrome),
            failed,
        })
    }
}

/// Replaces the `cache/` of the project laid out at `folder` with a link to
/// the project's page cache `kept`, so captures outlive the job.
pub(super) fn share_cache(folder: &Path, kept: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(kept)?;
    let cache = folder.join(CACHE_DIR);
    if cache.is_dir() && !cache.is_symlink() {
        std::fs::remove_dir(&cache)?;
    }
    std::os::unix::fs::symlink(std::path::absolute(kept)?, &cache)
}

/// How waiting for the worker's answer ended, short of a cancel.
pub(super) enum Waited {
    /// It answered.
    Answered(Answer),
    /// It went quiet first: it is not running.
    Quiet,
    /// The job's folder went, so no answer is coming (#1469).
    Gone,
    /// It was still at it when the caller's patience ran out.
    Late,
}

/// Waits for the answer to the job at `job`, for at most `patience` when one
/// is given; `Err` once `cancel` is tripped.
pub(super) fn wait(
    spool: &Spool,
    job: &Path,
    cancel: &Cancel,
    patience: Option<Duration>,
) -> Result<Waited, String> {
    let answer = job.join(ANSWER);
    let started = Instant::now();
    loop {
        if let Some(answer) = read::<Answer>(&answer) {
            return Ok(Waited::Answered(answer));
        }
        if cancel.is_cancelled() {
            return Err("stopped while its pages were being captured".to_owned());
        }
        if !job.is_dir() {
            return Ok(Waited::Gone);
        }
        if started.elapsed() > SILENCE && !worker_alive(spool) {
            return Ok(Waited::Quiet);
        }
        if patience.is_some_and(|patience| started.elapsed() > patience) {
            return Ok(Waited::Late);
        }
        std::thread::sleep(TICK);
    }
}

/// Whether the worker has looked at the spool lately.
pub(super) fn worker_alive(spool: &Spool) -> bool {
    let last: Option<u64> = std::fs::read_to_string(spool.alive())
        .ok()
        .and_then(|text| text.trim().parse().ok());
    last.is_some_and(|last| now().saturating_sub(last) <= SILENCE.as_secs())
}

/// The browser this server's captures are kept under: the one
/// [`CHROME_ENV`] names, which the image bakes in for both containers. Never
/// looked for anywhere else, and never downloaded.
#[expect(
    clippy::disallowed_methods,
    reason = "SCORSESE_CHROME names the capture worker's browser, not a secret"
)]
pub(super) fn browser() -> Result<Chrome, String> {
    let named = std::env::var_os(CHROME_ENV).ok_or_else(|| {
        format!(
            "this server has no page renderer ({CHROME_ENV} is not set), so pages are not drawn"
        )
    })?;
    Chrome::at(named).map_err(|error| error.to_string())
}

fn none_captured(requests: &[Wanted], why: &str, chrome: Option<Chrome>) -> Captured {
    Captured {
        chrome,
        failed: requests
            .iter()
            .map(|wanted| (wanted.request.page.clone(), why.to_owned()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spool whose worker said it was there a moment ago, and a job in it.
    fn asked(label: &str) -> (Spool, PathBuf) {
        let spool = Spool::new(crate::scratch::dir(label));
        let job = spool.still(1);
        std::fs::create_dir_all(&job).unwrap();
        std::fs::write(spool.alive(), now().to_string()).unwrap();
        (spool, job)
    }

    #[test]
    fn a_job_whose_folder_went_is_never_waited_on() {
        let (spool, job) = asked("dispatch-gone");
        std::fs::remove_dir_all(&job).unwrap();

        let waited = wait(&spool, &job, &Cancel::new(), None);

        assert!(matches!(waited, Ok(Waited::Gone)));
    }

    #[test]
    fn a_stopped_call_stops_waiting() {
        let (spool, job) = asked("dispatch-stopped");
        let cancel = Cancel::new();
        cancel.cancel();

        let waited = wait(&spool, &job, &cancel, None);

        assert!(waited.is_err());
    }

    #[test]
    fn past_its_patience_a_job_still_at_it_is_late() {
        let (spool, job) = asked("dispatch-late");

        let waited = wait(&spool, &job, &Cancel::new(), Some(Duration::ZERO));

        assert!(matches!(waited, Ok(Waited::Late)));
        assert!(job.is_dir(), "left to the launcher");
    }
}
