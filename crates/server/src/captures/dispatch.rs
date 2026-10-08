//! The server's side of the spool: a render job asks for its pages and waits.
//!
//! The job has laid its project out in its spool folder ([`Pages::folder`]).
//! This points that folder's `cache/` at the project's page cache, writes the
//! ask, and waits for the worker's answer — or for the job to be cancelled,
//! or for the worker to be plainly not running. The render then draws every
//! page from that cache and never starts a browser itself
//! ([`scorsese_render::Renderer::without_capturing`]).

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use scorsese_core::{CACHE_DIR, Project};
use scorsese_render::page::{CHROME_ENV, Chrome, Wanted};
use scorsese_render::{Cancel, Note};

use super::{ANSWER, ASK, Answer, Ask, Asked, PROJECT, Spool, now, publish, read};

/// How stale the worker's last look may be before the server stops waiting
/// for it: it looks at least every second, busy or not.
const SILENCE: Duration = Duration::from_secs(30);

/// How often the server looks for the answer.
const TICK: Duration = Duration::from_millis(250);

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
        if let Err(error) = self.share_cache() {
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
        let answer = self.wait(cancel)?;
        let Some(answer) = answer else {
            let why = "the page renderer is not running on this server; the pages are \
                       drawn once it is";
            return Ok(none_captured(requests, why, Some(chrome)));
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

    /// Replaces the laid-out `cache/` with a link to the project's page cache,
    /// so captures outlive the job.
    fn share_cache(&self) -> std::io::Result<()> {
        let kept = &self.cache;
        std::fs::create_dir_all(kept)?;
        let cache = self.folder().join(CACHE_DIR);
        if cache.is_dir() && !cache.is_symlink() {
            std::fs::remove_dir(&cache)?;
        }
        std::os::unix::fs::symlink(std::path::absolute(kept)?, &cache)
    }

    /// The answer, `None` if the worker went quiet first, or `Err` once the
    /// job is cancelled.
    fn wait(&self, cancel: &Cancel) -> Result<Option<Answer>, String> {
        let answer = self.job_folder().join(ANSWER);
        let started = Instant::now();
        loop {
            if let Some(answer) = read::<Answer>(&answer) {
                return Ok(Some(answer));
            }
            if cancel.is_cancelled() {
                return Err("stopped while its pages were being captured".to_owned());
            }
            if started.elapsed() > SILENCE && !self.worker_alive() {
                return Ok(None);
            }
            std::thread::sleep(TICK);
        }
    }

    fn worker_alive(&self) -> bool {
        let last: Option<u64> = std::fs::read_to_string(self.spool.alive())
            .ok()
            .and_then(|text| text.trim().parse().ok());
        last.is_some_and(|last| now().saturating_sub(last) <= SILENCE.as_secs())
    }
}

/// The browser this server's captures are kept under: the one
/// [`CHROME_ENV`] names, which the image bakes in for both containers. Never
/// looked for anywhere else, and never downloaded.
fn browser() -> Result<Chrome, String> {
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

/// What a render said, in the words its owner reads: every note, with a page
/// that showed its card saying why the worker did not capture it rather than
/// the renderer's "not captured yet".
pub fn said(notes: &[Note], project: &Project, failed: &HashMap<String, String>) -> Vec<String> {
    let mut said: Vec<String> = Vec::new();
    for note in notes {
        let note = match note {
            Note::PageNotCaptured { clip, asset, .. } => {
                let page = project
                    .assets
                    .iter()
                    .find(|a| a.id.as_str() == asset)
                    .and_then(|a| a.path.as_ref());
                match page.and_then(|path| failed.get(path.as_str())) {
                    Some(why) => Note::PageNotCaptured {
                        clip: clip.clone(),
                        asset: asset.clone(),
                        reason: why.clone(),
                    },
                    None => note.clone(),
                }
            }
            other => other.clone(),
        };
        let line = note.to_string();
        if !said.contains(&line) {
            said.push(line);
        }
    }
    said
}

#[cfg(test)]
mod tests {
    use scorsese_core::{Asset, AssetId, AssetKind, ProjectPath};

    use super::*;

    #[test]
    fn a_page_left_as_its_card_says_why_the_worker_did_not_capture_it() {
        let mut project = Project::new("p", scorsese_core::Fps::THIRTY);
        project.assets.push(Asset::imported(
            AssetId::new("title"),
            AssetKind::Html,
            ProjectPath::new("pages/title.html"),
        ));
        let card = |reason: &str| Note::PageNotCaptured {
            clip: "c1".into(),
            asset: "title".into(),
            reason: reason.into(),
        };
        let warned = Note::PageWarning {
            asset: "title".into(),
            warning: "it asked for the internet".into(),
        };
        let failed = HashMap::from([("pages/title.html".to_owned(), "stopped".to_owned())]);
        let notes = [card("not captured yet"), warned.clone(), warned.clone()];

        let said = said(&notes, &project, &failed);

        assert_eq!(said, [card("stopped").to_string(), warned.to_string()]);
        let none = said_with_nothing_failed(&project);
        assert!(none[0].ends_with("not captured yet"), "{none:?}");
    }

    fn said_with_nothing_failed(project: &Project) -> Vec<String> {
        let card = Note::PageNotCaptured {
            clip: "c1".into(),
            asset: "title".into(),
            reason: "not captured yet".into(),
        };
        said(&[card], project, &HashMap::new())
    }
}
