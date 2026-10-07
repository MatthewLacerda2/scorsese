//! The page under the playhead, captured before the rest of it (#875).
//!
//! A whole page is captured from its first frame, so the frame being looked at
//! near the end of a long one waits for every frame before it: a still at 85 s
//! of a 90 s page took 254 s that way (#872). Captured on its own, the same
//! instant takes about 8 s — the page's clock is run ahead without drawing and
//! only a handful of frames around the instant are drawn, kept in the page's
//! cache as a piece of it, which the preview already reads
//! ([`scorsese_render::page::cached_frames`]).
//!
//! So whenever the picture shows a page whose instant under the playhead has no
//! capture, this captures that instant, on a thread of its own, beside the
//! whole-page capture rather than behind it: it is a still of the preview's own
//! settings drawn by a renderer that *does* capture, which is exactly the
//! stretch a still asks for. One at a time; the playhead moving meanwhile is
//! caught by the picture asking again once this lands, since landing redraws it.

use std::sync::{Arc, Mutex, PoisonError};

use scorsese_core::{Frames, Project};
use scorsese_render::page::Chrome;
use scorsese_render::preview::{self, Preview, Proxies};
use scorsese_render::{Quality, RenderSettings, Renderer, Tools};

use crate::project::Open;

/// What a capture at the playhead came to: the browser it drew with, or why it
/// could not.
type Outcome = Result<Chrome, String>;

/// The capture at the playhead, while one runs, and what the last one came to.
#[derive(Default)]
pub(super) struct Glancer {
    /// Filled by the thread when it is done.
    running: Option<Arc<Mutex<Option<Outcome>>>>,
    /// The instant and quality last captured at — so an instant that could not
    /// be captured is not tried again on every redraw that still shows a card.
    last: Option<(Frames, Quality)>,
    /// Why the last one failed, when it did.
    failed: Option<String>,
}

impl Glancer {
    /// Captures the pages at `at`, at `quality`'s raster, unless one capture is
    /// already running or this instant was just tried.
    pub(super) fn start(&mut self, open: &Open, at: Frames, quality: Quality) {
        if !self.wants(at, quality) {
            return;
        }
        self.last = Some((at, quality));
        let outcome = Arc::new(Mutex::new(None));
        let shared = Arc::clone(&outcome);
        let (project, root) = (open.project.clone(), open.root.clone());
        std::thread::spawn(move || {
            let captured = capture_at(&project, &root, at, quality);
            *shared.lock().unwrap_or_else(PoisonError::into_inner) = Some(captured);
        });
        self.running = Some(outcome);
    }

    /// Whether a capture at `at` and `quality` should start now.
    fn wants(&self, at: Frames, quality: Quality) -> bool {
        self.running.is_none() && self.last != Some((at, quality))
    }

    /// The browser a capture drew with, once one has landed since the last
    /// time this was asked — which is when the picture should be drawn again.
    pub(super) fn landed(&mut self) -> Option<Chrome> {
        let outcome = self
            .running
            .as_ref()?
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()?;
        self.running = None;
        match outcome {
            Ok(chrome) => {
                self.failed = None;
                Some(chrome)
            }
            Err(why) => {
                self.failed = Some(why);
                None
            }
        }
    }

    /// Whether a capture is running right now.
    pub(super) fn running(&self) -> bool {
        self.running.is_some()
    }

    /// The document changed: an instant tried before may capture now.
    pub(super) fn forget(&mut self) {
        self.last = None;
    }

    /// What to say about it under the preview, when there is anything.
    pub(super) fn status(&self) -> Option<String> {
        if self.running() {
            return Some("capturing the page at the playhead".to_owned());
        }
        self.failed
            .as_ref()
            .map(|why| format!("the page at the playhead could not be captured — {why}"))
    }
}

/// The thread: the preview's own still at `at`, by a renderer that captures
/// the pages it shows — only their frames around `at` (#809).
fn capture_at(project: &Project, root: &std::path::Path, at: Frames, quality: Quality) -> Outcome {
    let tools = Tools::discover().map_err(|error| error.to_string())?;
    let chrome = Chrome::discover().map_err(|error| error.to_string())?;
    // The preview's settings, so the capture is the one it reads.
    let settings =
        RenderSettings::new(super::super::quality::raster(quality), project.timeline_fps);
    let proxies = Proxies::made_in(&preview::folder(root), project);
    Renderer::new(&tools, settings)
        .with_preview(Preview::new(quality).with_proxies(proxies))
        .with_chrome(chrome.clone())
        .still(project, root, at)
        .map_err(|error| error.to_string())?;
    Ok(chrome)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An instant is captured once: a redraw still showing its card — a page
    /// that could not be captured — does not start it again, while another
    /// instant, another quality or a changed document does.
    #[test]
    fn an_instant_is_tried_once_until_something_changes() {
        let mut glancer = Glancer::default();
        let at = Frames(90);
        assert!(glancer.wants(at, Quality::Half));
        glancer.last = Some((at, Quality::Half));
        assert!(!glancer.wants(at, Quality::Half));
        assert!(glancer.wants(Frames(91), Quality::Half));
        assert!(glancer.wants(at, Quality::Full));
        glancer.forget();
        assert!(glancer.wants(at, Quality::Half));
    }

    /// One at a time: while a capture runs, no other starts, and its outcome
    /// is taken once — a failure said under the preview, a browser handed on.
    #[test]
    fn one_runs_at_a_time_and_lands_once() {
        let mut glancer = Glancer::default();
        let outcome = Arc::new(Mutex::new(None));
        glancer.running = Some(Arc::clone(&outcome));
        assert!(!glancer.wants(Frames(1), Quality::Half));
        assert_eq!(glancer.landed(), None, "nothing landed yet");
        assert!(glancer.running());
        *outcome.lock().expect("lock") = Some(Err("no browser".to_owned()));
        assert_eq!(glancer.landed(), None);
        assert!(!glancer.running());
        assert_eq!(
            glancer.status().as_deref(),
            Some("the page at the playhead could not be captured — no browser")
        );
    }
}
