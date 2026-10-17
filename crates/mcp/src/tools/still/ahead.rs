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

use scorsese_render::page::{Chrome, Wanted};

/// Whoever captures a still's pages before it is drawn, so the still itself
/// never starts a browser.
pub trait PagesAhead: Send + Sync {
    /// Captures `wanted` for the project laid out at `project_root`, into its
    /// `cache/`, where the still then reads them. Never fails: a page it could
    /// not capture is named in [`Ahead::failed`], and shows its slug card.
    fn capture(&self, project_root: &Path, wanted: &[Wanted]) -> Ahead;
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
        fn capture(&self, _: &Path, wanted: &[Wanted]) -> Ahead {
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
        // The frames around each instant, and no more (#809).
        let stretches: Vec<_> = asked.iter().map(|one| one.frames.clone()).collect();
        assert_eq!(stretches, [13..18, 16..21], "{asked:?}");
        assert!(asked.iter().all(|one| one.request.page == "pages/p.html"));
        let said = &reply.parts[0].text;
        assert!(said.contains("the launcher said no"), "{said}");
        assert!(!dir.join("cache/pages").exists(), "nothing captured here");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
