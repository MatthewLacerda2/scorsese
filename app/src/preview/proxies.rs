//! Making the open project's proxies, in the background (#542).
//!
//! A proxy is a small copy of a heavy video that a reduced-quality preview
//! decodes instead of the original — [`scorsese_render::preview`] has what one
//! is. Here they are made **lazily**: the first time a project is shown at a
//! quality that reads them, every video worth one that has none gets one, one
//! at a time on a thread of its own, into the project's rebuildable `cache/`.
//! Until each is made the preview reads the original, which is correct and
//! only slower; the moment one lands, the picture is drawn again from it.
//!
//! Lazily rather than at import, because importing is the CLI's and the MCP
//! server's as much as the window's, and neither of them ever previews: a
//! transcode for an agent that will never watch is a machine's worth of work
//! for nothing. The window is the only thing here that plays the cut.
//!
//! One at a time, because a transcode is as heavy as a render and the machine
//! this runs on is also serving other people's renders.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use scorsese_core::Project;
use scorsese_render::{Tools, preview};

use crate::project::Open;

/// The proxies being made for the open project, if any are.
#[derive(Default)]
pub(super) struct Maker {
    /// The thread's shared state, while one is running or has just finished.
    job: Option<Job>,
    /// Whether this document has been looked through for missing proxies.
    /// Cleared when it changes, and looked at again once no job is running —
    /// so a video added while proxies are being made gets one after.
    looked: bool,
    /// How many the preview has already been told about, so a new one is
    /// news exactly once.
    seen: usize,
}

/// A thread making proxies, and how far it got.
struct Job {
    progress: Arc<Mutex<Progress>>,
    stop: Arc<AtomicBool>,
}

/// How far a thread got.
#[derive(Debug, Clone, Default)]
struct Progress {
    made: usize,
    total: usize,
    failed: Option<String>,
    finished: bool,
}

impl Maker {
    /// Starts making whatever `open` is missing, unless a thread already is or
    /// this document has been looked through.
    pub(super) fn ensure(&mut self, open: &Open) {
        if self.running() || self.looked {
            return;
        }
        self.looked = true;
        let wanted = missing(&open.project, &open.root);
        if wanted.is_empty() {
            return;
        }
        let progress = Arc::new(Mutex::new(Progress {
            total: wanted.len(),
            ..Progress::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (shared, stopping) = (Arc::clone(&progress), Arc::clone(&stop));
        std::thread::spawn(move || make_all(&wanted, &shared, &stopping));
        self.job = Some(Job { progress, stop });
        self.seen = 0;
    }

    /// The document changed: look through it again once nothing is running.
    pub(super) fn document_changed(&mut self) {
        self.looked = false;
    }

    /// Whether a proxy has landed since the last time this was asked — which
    /// is when the picture on screen should be drawn again from it.
    pub(super) fn landed(&mut self) -> bool {
        let made = self.progress().map_or(0, |progress| progress.made);
        let news = made > self.seen;
        self.seen = made;
        news
    }

    /// Whether a thread is making proxies right now.
    pub(super) fn running(&self) -> bool {
        self.progress().is_some_and(|progress| !progress.finished)
    }

    /// What to say about it under the preview, when there is anything.
    pub(super) fn status(&self) -> Option<String> {
        let progress = self.progress()?;
        if let Some(failed) = progress.failed {
            return Some(format!("a proxy could not be made — {failed}"));
        }
        (!progress.finished).then(|| {
            format!(
                "making proxies: {} of {}",
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

impl Drop for Maker {
    /// A thread making another project's proxies stops after the one it is on.
    fn drop(&mut self) {
        if let Some(job) = &self.job {
            job.stop.store(true, Ordering::Relaxed);
        }
    }
}

/// Every video in `project` worth a proxy that does not have one yet: its
/// file, and where its proxy goes.
fn missing(project: &Project, root: &std::path::Path) -> Vec<(PathBuf, PathBuf)> {
    let folder = preview::folder(root);
    project
        .assets
        .iter()
        .filter_map(|asset| {
            let media = asset.media.as_ref()?;
            let sha256 = asset.sha256.as_ref()?;
            let source = asset.path.as_ref()?.resolve(root);
            let out = folder.join(preview::file_name(sha256));
            (preview::worth_making(asset.kind, media) && source.is_file() && !out.is_file())
                .then_some((source, out))
        })
        .collect()
}

/// The thread: each proxy in turn, until done or told to stop.
fn make_all(wanted: &[(PathBuf, PathBuf)], progress: &Mutex<Progress>, stop: &AtomicBool) {
    let update = |change: &dyn Fn(&mut Progress)| {
        change(&mut progress.lock().unwrap_or_else(PoisonError::into_inner));
    };
    let tools = match Tools::discover() {
        Ok(tools) => tools,
        Err(error) => {
            let why = error.to_string();
            return update(&|progress| {
                progress.failed = Some(why.clone());
                progress.finished = true;
            });
        }
    };
    for (source, out) in wanted {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match preview::make(&tools, source, out) {
            Ok(()) => update(&|progress| progress.made += 1),
            Err(error) => {
                let why = error.to_string();
                update(&|progress| progress.failed = Some(why.clone()));
            }
        }
    }
    update(&|progress| progress.finished = true);
}
