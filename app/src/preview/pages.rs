//! Capturing the open project's web pages, in the background (#776).
//!
//! An `html` clip is drawn from a capture of its page, and making one is slow —
//! a minute of page is minutes of browser (#606). So the preview never makes
//! one: it draws pages only from captures already in `cache/`
//! ([`scorsese_render::Renderer::without_capturing`]), as their slug card until
//! then, and this captures them on a thread of its own — one at a time, like
//! [`super::proxies`], whose pattern this is. The moment one lands, the picture
//! is drawn again from it, and scrubbing over it from then on costs what
//! scrubbing a video costs.
//!
//! **At the preview's raster.** A page's viewport is 1080 CSS pixels on its
//! shorter side at any raster, so a smaller capture lays out identically and
//! only has fewer pixels. Measured on a 4-core machine, ten seconds of page:
//! half the delivery raster captured 2.7–2.9× faster than full and took a third
//! of the disk (a full-frame animated page: 59 s / 210 MB at full, 20 s / 70 MB
//! at half). Quarter cost exactly what half did — Chromium does not scale a
//! page below half — so it saves nothing more, but costs nothing either.
//!
//! The first capture on a machine with no browser downloads one
//! ([`fetch_on_first_use`]); the line under the preview says how far it got.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use scorsese_core::{AssetKind, Project};
use scorsese_providers::chromium::{self, Fetching};
use scorsese_render::page::{self, Chrome, Supply};
use scorsese_render::{Quality, RenderSettings, Renderer, Tools};

use crate::project::Open;

/// How far the page renderer's download has got, while one runs.
static FETCHING: Mutex<Option<Fetching>> = Mutex::new(None);

/// Lets this program download the page renderer the first time a page needs
/// drawing — the preview's captures and a render's alike — and shows how far
/// it got under the preview.
pub fn fetch_on_first_use() {
    page::supply(Supply::new(chromium::installed, || {
        let fetched = chromium::fetch(&mut |at| *fetching() = Some(at));
        *fetching() = None;
        fetched.map_err(|error| error.to_string())
    }));
}

fn fetching() -> std::sync::MutexGuard<'static, Option<Fetching>> {
    FETCHING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The pages being captured for the open project, if any are.
#[derive(Default)]
pub(super) struct Capturer {
    /// The thread's shared state, while one is running or has just finished.
    job: Option<Job>,
    /// The quality this document was last looked through at. Cleared when it
    /// changes; a different quality is a different raster, so other captures.
    looked: Option<Quality>,
    /// The browser the captures are made with, once a thread has found it —
    /// the preview needs its version to find them in the cache.
    chrome: Option<Chrome>,
    /// How many captures the preview has already been told about.
    seen: usize,
}

struct Job {
    progress: Arc<Mutex<Progress>>,
    stop: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Default)]
struct Progress {
    chrome: Option<Chrome>,
    made: usize,
    total: usize,
    failed: Option<String>,
    finished: bool,
}

impl Capturer {
    /// Starts capturing whatever pages `open` shows at `quality`, unless a
    /// thread already is or this document was looked through at it.
    pub(super) fn ensure(&mut self, open: &Open, quality: Quality) {
        if self.running() || self.looked == Some(quality) {
            return;
        }
        self.looked = Some(quality);
        if !has_pages(&open.project) {
            return;
        }
        let progress = Arc::new(Mutex::new(Progress::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (shared, stopping) = (Arc::clone(&progress), Arc::clone(&stop));
        let (project, root) = (open.project.clone(), open.root.clone());
        let settings = RenderSettings::new(super::quality::raster(quality), project.timeline_fps);
        std::thread::spawn(move || {
            capture_all(&project, &root, settings, &shared, &stopping);
        });
        self.job = Some(Job { progress, stop });
        self.seen = 0;
    }

    /// The document changed: look through it again once nothing is running.
    pub(super) fn document_changed(&mut self) {
        self.looked = None;
    }

    /// The browser to read captures with, once one has been found.
    pub(super) fn chrome(&self) -> Option<&Chrome> {
        self.chrome.as_ref()
    }

    /// Whether a capture landed, or the browser was found, since the last
    /// time this was asked — which is when the picture should be drawn again.
    pub(super) fn landed(&mut self) -> bool {
        let Some(progress) = self.progress() else {
            return false;
        };
        let found = self.chrome.is_none() && progress.chrome.is_some();
        if found {
            self.chrome = progress.chrome;
        }
        let news = progress.made > self.seen;
        self.seen = progress.made;
        found || news
    }

    /// Whether a thread is capturing right now.
    pub(super) fn running(&self) -> bool {
        self.progress().is_some_and(|progress| !progress.finished)
    }

    /// What to say about it under the preview, when there is anything.
    pub(super) fn status(&self) -> Option<String> {
        if let Some(at) = *fetching() {
            return Some(fetching_said(at));
        }
        let progress = self.progress()?;
        if let Some(failed) = progress.failed {
            return Some(format!("a page could not be captured — {failed}"));
        }
        (!progress.finished && progress.total > 0).then(|| {
            format!(
                "capturing pages: {} of {}",
                progress.made + 1,
                progress.total
            )
        })
    }

    fn progress(&self) -> Option<Progress> {
        let job = self.job.as_ref()?;
        Some(
            job.progress
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone(),
        )
    }
}

impl Drop for Capturer {
    /// A thread capturing another project's pages stops after the one it is on.
    fn drop(&mut self) {
        if let Some(job) = &self.job {
            job.stop.store(true, Ordering::Relaxed);
        }
    }
}

/// How far the download has got, in the words under the preview.
fn fetching_said(at: Fetching) -> String {
    let megabytes = |bytes: u64| bytes.div_ceil(1_000_000);
    match at.total {
        Some(total) => format!(
            "fetching the page renderer, once: {} of {} MB",
            megabytes(at.received),
            megabytes(total)
        ),
        None => format!(
            "fetching the page renderer, once: {} MB",
            megabytes(at.received)
        ),
    }
}

/// Whether `project` has a page in it at all — looked at before a thread is
/// started, so a project without one never starts one.
fn has_pages(project: &Project) -> bool {
    project
        .assets
        .iter()
        .any(|asset| asset.kind == AssetKind::Html)
}

/// The thread: find the browser (downloading it the first time), then each
/// capture in turn, until done or told to stop.
fn capture_all(
    project: &Project,
    root: &std::path::Path,
    settings: RenderSettings,
    progress: &Mutex<Progress>,
    stop: &AtomicBool,
) {
    let update = |change: &mut dyn FnMut(&mut Progress)| {
        change(&mut progress.lock().unwrap_or_else(PoisonError::into_inner));
    };
    let fail = |why: String| {
        update(&mut |progress| {
            progress.failed = Some(why.clone());
            progress.finished = true;
        });
    };
    let tools = match Tools::discover() {
        Ok(tools) => tools,
        Err(error) => return fail(error.to_string()),
    };
    let requests = match Renderer::new(&tools, settings).page_requests(project) {
        Ok(requests) => requests,
        Err(error) => return fail(error.to_string()),
    };
    if requests.is_empty() {
        return update(&mut |progress| progress.finished = true);
    }
    update(&mut |progress| progress.total = requests.len());
    let chrome = match Chrome::discover() {
        Ok(chrome) => chrome,
        Err(error) => return fail(error.to_string()),
    };
    update(&mut |progress| progress.chrome = Some(chrome.clone()));
    for request in &requests {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match page::capture(&chrome, &tools, root, request) {
            Ok(_) => update(&mut |progress| progress.made += 1),
            Err(error) => {
                let why = error.to_string();
                update(&mut |progress| progress.failed = Some(why.clone()));
            }
        }
    }
    update(&mut |progress| progress.finished = true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_download_says_how_far_it_got() {
        let at = |received, total| Fetching { received, total };
        assert_eq!(
            fetching_said(at(45_000_000, Some(121_000_000))),
            "fetching the page renderer, once: 45 of 121 MB"
        );
        assert_eq!(
            fetching_said(at(0, None)),
            "fetching the page renderer, once: 0 MB"
        );
    }

    /// A project with no page never starts a thread, so it never looks for a
    /// browser, let alone downloads one.
    #[test]
    fn only_a_project_with_a_page_captures() {
        use scorsese_core::{Asset, AssetId, Fps, ProjectPath};
        let mut project = Project::new("x", Fps::THIRTY);
        assert!(!has_pages(&project));
        project.assets.push(Asset::imported(
            AssetId::new("title"),
            AssetKind::Html,
            ProjectPath::new("pages/title.html"),
        ));
        assert!(has_pages(&project));
    }
}
