//! A still whose pages are captured somewhere else first (#1434).
//!
//! Locally `still` captures a page on screen itself, in a browser it starts.
//! The web app must not: its server's container holds the database's password
//! and reaches the internet, so a user's page is only ever drawn in a capture
//! container of its own (#778). There, `still` is handed a [`PagesAhead`],
//! which captures what the asked instants show before anything is drawn, and
//! the frames are drawn by a renderer that never starts a browser
//! ([`scorsese_render::Renderer::without_capturing`]). The tool, its arguments
//! and its description are the same either way: who captures is not the
//! caller's business.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use scorsese_render::Cancel;
use scorsese_render::page::{Chrome, Wanted};

/// Whoever captures a still's pages before it is drawn, so the still itself
/// never starts a browser.
pub trait PagesAhead: Send + Sync {
    /// Captures `wanted` for the project laid out at `project_root`, into its
    /// `cache/`, where the still then reads them. Never fails: a page it could
    /// not capture is named in [`Ahead::failed`], and shows its slug card.
    /// Stops waiting once `cancel`, the call's, is tripped.
    fn capture(&self, project_root: &Path, wanted: &[Wanted], cancel: &Cancel) -> Ahead;
}

/// `wanted` with the stretches of one page that overlap or touch made one, in
/// the order each page was first wanted (#1469). Each stretch is a capture of
/// its own wherever it is captured — on the web a container — so five cells
/// of a sheet a second apart on one page are one capture, not five. Stretches
/// apart are left apart: the frames between them would be drawn for nothing.
pub(super) fn merged(wanted: Vec<Wanted>) -> Vec<Wanted> {
    let mut sorted: Vec<(usize, Wanted)> = wanted.into_iter().enumerate().collect();
    // Stable, so the first of each page keeps its place among the pages.
    sorted.sort_by_key(|(_, one)| one.frames.start);
    let mut kept: Vec<(usize, Wanted)> = Vec::new();
    for (order, one) in sorted {
        let joins = kept.iter_mut().find(|(_, held)| {
            held.request == one.request
                && one.frames.start <= held.frames.end
                && held.frames.start <= one.frames.end
        });
        match joins {
            Some((first, held)) => {
                held.frames =
                    held.frames.start.min(one.frames.start)..held.frames.end.max(one.frames.end);
                *first = (*first).min(order);
            }
            None => kept.push((order, one)),
        }
    }
    kept.sort_by_key(|(order, _)| *order);
    kept.into_iter().map(|(_, one)| one).collect()
}

/// What came of capturing ahead.
#[derive(Debug, Default)]
pub struct Ahead {
    /// The build the captures are kept under, or `None` when there is none to
    /// capture with and every page shows its card.
    pub chrome: Option<Chrome>,
    /// Why each page path that has no capture has none.
    pub failed: HashMap<String, String>,
    /// True when some page is still being captured, so its card says so
    /// ([`scorsese_render::Renderer::capture_underway`]).
    pub underway: bool,
}

/// `still`, with its pages captured by `pages` rather than by itself.
pub fn still_with_pages_ahead(pages: Arc<dyn PagesAhead>) -> Box<dyn crate::Tool> {
    Box::new(super::Still { ahead: Some(pages) })
}

/// A still handed a [`PagesAhead`] asks it for the page on screen, draws
/// without capturing, and says why the page was not captured in its words.
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use scorsese_core::{
        Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, Project, ProjectPath, Track, TrackId,
        TrackKind,
    };
    use serde_json::json;

    use super::*;

    /// Captures nothing, and remembers what it was asked.
    #[derive(Default)]
    struct Refusing(Mutex<Vec<Wanted>>);

    impl PagesAhead for Refusing {
        fn capture(&self, _: &Path, wanted: &[Wanted], _: &Cancel) -> Ahead {
            self.0.lock().unwrap().extend_from_slice(wanted);
            let failed = wanted
                .iter()
                .map(|one| (one.request.page.clone(), "the launcher said no".to_owned()))
                .collect();
            Ahead {
                failed,
                ..Ahead::default()
            }
        }
    }

    fn showing_a_page(dir: &Path) {
        std::fs::create_dir_all(dir.join("pages")).unwrap();
        std::fs::write(dir.join("pages/p.html"), "<body></body>").unwrap();
        let mut project = Project::new("ahead", Fps::THIRTY);
        project.assets.push(Asset::imported(
            AssetId::new("p"),
            AssetKind::Html,
            ProjectPath::new("pages/p.html"),
        ));
        let mut track = Track::new(TrackId::new("v1"), TrackKind::Video);
        let clip = Clip::new(ClipId::new("c"), AssetId::new("p"), Frames(0), Frames(30));
        track.clips.push(clip);
        project.tracks.push(track);
        std::fs::write(dir.join("project.json"), project.to_json().unwrap()).unwrap();
    }

    #[test]
    fn the_page_is_asked_of_whoever_captures_ahead_and_its_reason_is_said() {
        let dir = crate::scratch::fresh("scorsese-mcp-ahead", "");
        let _ = std::fs::remove_dir_all(&dir);
        showing_a_page(&dir);
        let pages = Arc::new(Refusing::default());
        let still = still_with_pages_ahead(pages.clone());
        let at = json!({ "project": dir, "at": ["0.5s", "0.6s"], "resolution": "64x36" });

        let reply = still.call(&at).expect("a still");

        let asked = pages.0.lock().unwrap();
        // The frames around each instant, and no more (#809), the two
        // overlapping stretches asked as one (#1469).
        let stretches: Vec<_> = asked.iter().map(|one| one.frames.clone()).collect();
        assert_eq!(stretches, vec![13..21_u64], "{asked:?}");
        assert!(asked.iter().all(|one| one.request.page == "pages/p.html"));
        let said = &reply.parts[0].text;
        assert!(said.contains("the launcher said no"), "{said}");
        assert!(!dir.join("cache/pages").exists(), "nothing captured here");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn of(page: &str, frames: std::ops::Range<u64>) -> Wanted {
        let request = scorsese_render::page::Request {
            page: page.to_owned(),
            resolution: scorsese_render::Resolution::new(64, 36).unwrap(),
            fps: Fps::THIRTY,
            duration: 10.0,
            clips: std::collections::BTreeMap::new(),
            words: std::collections::BTreeMap::new(),
        };
        Wanted::new(request, frames)
    }

    #[test]
    fn a_pages_overlapping_and_touching_stretches_are_one_capture() {
        let wanted = vec![
            of("b.html", 40..45),
            of("a.html", 13..18),
            of("b.html", 30..41),
            of("a.html", 18..23),
            of("a.html", 16..21),
        ];

        let merged = merged(wanted);

        assert_eq!(merged, [of("b.html", 30..45), of("a.html", 13..23)]);
    }

    #[test]
    fn stretches_apart_or_of_other_pages_stay_apart() {
        let wanted = vec![
            of("a.html", 13..18),
            of("a.html", 100..105),
            of("b.html", 14..19),
        ];

        assert_eq!(merged(wanted.clone()), wanted);
    }
}
