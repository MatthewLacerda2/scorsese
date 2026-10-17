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
