//! Web pages as a frame source: an `html` clip drawn by a headless browser.
//!
//! A sibling of the ffmpeg decode, and wired the same way. ffmpeg turns a video
//! file into frames; this turns a page into a video file — once, into
//! `cache/` — and from then on the page *is* a video with alpha to the rest of
//! the render, read by the ordinary decode path with `source_in` and `speed`
//! meaning what they mean for footage (#789).
//!
//! **[`capture`] is its own step.** A render calls it for each page it needs
//! before drawing (`Renderer` does this itself), and anything else may call it
//! ahead of time: the web app captures in a different container from the one
//! that renders (#778), and a desktop app may want to capture while the person
//! is still editing (#776). [`cached`] is the half that needs no browser —
//! whether a capture is already there for a request.
//!
//! What a page is told, and the clock it runs on, are [`Request`]'s and
//! `clock.js`'s; how it is served and kept offline is `origin`'s; what it
//! loaded and what went wrong is `visitor`'s; what its layout gets wrong is
//! `layout`'s; the cache is `cache`'s.
//!
//! Boundary: `scorsese-core` never learns a browser exists. Everything that
//! knows one does is in this folder.

mod browser;
mod cache;
mod capture;
mod cdp;
mod find;
mod fonts;
mod icons;
mod layout;
mod origin;
mod request;
mod supply;
mod visitor;

use std::path::{Path, PathBuf};

pub use browser::{CHROME_ENV, Chrome, ChromeError, NO_SANDBOX, NO_SANDBOX_ENV};
pub use cdp::CdpError;
pub use origin::SHIPPED_ORIGIN;
pub use request::Request;
pub use supply::{Supply, supply};

pub(crate) use find::find;

use crate::error::RenderError;
use crate::tools::Tools;

/// The version of the capture method itself — the clock shim, the page
/// contract, the flags, the shipped libraries and the file the frames are kept
/// in — and what a capture reports. Part of every capture's cache key, so
/// **bump it** when any of those changes what a page's frames look like or
/// what is said about them, and every capture is redone. 2: layout notes
/// (#813), which a capture kept from before them would never say. 3: no
/// WebSocket or WebRTC traffic, and a note when a page tries (#839), so a
/// page that drew what the network told it is drawn again without it.
pub const PAGE_VERSION: u32 = 3;

/// A capture, ready to be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    /// The lossless video with alpha holding the page's frames.
    pub file: PathBuf,
    /// What the page did that the author should hear about: a request refused
    /// because pages render offline, a WebSocket or WebRTC connection it opened
    /// (which reached nothing), a file it asked for that is not in the
    /// project, a script that threw, text laid out off the frame or over other
    /// text. Said again on every reuse of the capture.
    pub warnings: Vec<String>,
}

/// The capture `request` already has, if it is there and still describes the
/// project as it is now. Needs no browser — only the version of the one that
/// would capture it, since a capture by another build is another capture.
pub fn cached(project_root: &Path, request: &Request, chrome_version: &str) -> Option<Captured> {
    let slot = cache::slot(project_root, request, chrome_version);
    cache::fresh(&slot, project_root).map(|record| Captured {
        file: slot.join(cache::FRAMES),
        warnings: record.warnings,
    })
}

/// The page's frames for `request`, from the cache when they are there and
/// fresh, and captured with `chrome` when they are not.
pub fn capture(
    chrome: &Chrome,
    tools: &Tools,
    project_root: &Path,
    request: &Request,
) -> Result<Captured, PageError> {
    if let Some(captured) = cached(project_root, request, chrome.version()) {
        return Ok(captured);
    }
    let slot = cache::slot(project_root, request, chrome.version());
    std::fs::create_dir_all(&slot)?;
    let fonts = cache::fonts(project_root)?;
    let file = slot.join(cache::FRAMES);
    // Written beside and moved into place, so an interrupted capture never
    // leaves frames that look finished — under a name of this process's own, so
    // two renders of one project capturing the same page cannot interleave.
    let partial = slot.join(format!("frames.{}.partial.mkv", std::process::id()));
    let heard = capture::run(chrome, tools, project_root, &fonts, request, &partial)?;
    std::fs::rename(&partial, &file)?;
    let record = cache::Record {
        loaded: heard.loaded,
        warnings: heard.warnings,
    };
    cache::keep(&slot, &record)?;
    Ok(Captured {
        file,
        warnings: record.warnings,
    })
}

/// Why a page could not be captured. Never a failed render: the clip shows its
/// slug card instead, and the reason goes on the report.
#[derive(Debug, thiserror::Error)]
pub enum PageError {
    /// No browser to capture with.
    #[error(transparent)]
    Chrome(#[from] ChromeError),
    /// The browser stopped answering, or refused something.
    #[error(transparent)]
    Protocol(#[from] CdpError),
    /// The page itself would not load.
    #[error("the page would not load: {0}")]
    Load(String),
    /// The browser drew a frame and handed back no picture of it.
    #[error("the browser returned no picture for a frame")]
    NoPicture,
    /// Encoding the frames failed.
    #[error(transparent)]
    Render(#[from] RenderError),
    /// The cache could not be written.
    #[error("writing the capture to cache/: {0}")]
    Io(#[from] std::io::Error),
}
