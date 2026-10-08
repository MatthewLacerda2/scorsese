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
//! **Only the frames asked for are drawn** (#809). [`capture_frames`] captures a
//! stretch of the page — the few frames around a still, a clip's stretch from
//! its `source_in` — by running its clock ahead to the first of them without
//! drawing (`capture`'s), and a long stretch in pieces at once (`pieces`'). A
//! still 85 s into a 90 s page draws a handful of frames, not 2,550. The
//! frames are the ones a capture from the first frame would have drawn; what
//! is skipped is drawing the ones nobody looks at.
//!
//! What a page is told, and the clock it runs on, are [`Request`]'s and
//! `clock.js`'s; where the timeline's clips are, and which of them it read,
//! `told`'s (#810); how it is served and kept offline is `origin`'s; what it
//! loaded and what went wrong is `visitor`'s; what its layout gets wrong is
//! `layout`'s; the cache is `cache`'s.
//!
//! Boundary: `scorsese-core` never learns a browser exists. Everything that
//! knows one does is in this folder.

mod browser;
mod cache;
mod capture;
mod cdp;
mod encoder;
mod find;
mod fonts;
mod icons;
mod layout;
mod origin;
mod pieces;
mod request;
#[cfg(test)]
mod same_frames;
mod supply;
mod told;
mod visitor;

use std::ops::Range;
use std::path::{Path, PathBuf};

pub use browser::{CHROME_ENV, Chrome, ChromeError, NO_SANDBOX, NO_SANDBOX_ENV};
pub use cdp::CdpError;
pub use origin::SHIPPED_ORIGIN;
pub use request::Request;
pub use supply::{Supply, supply};
pub use told::Span;

pub(crate) use find::find;

use crate::error::RenderError;
use crate::tools::Tools;

/// The version of the capture method itself — the clock shim, the page
/// contract, the flags and the file the frames are kept in — and what a
/// capture reports. Not the shipped libraries, from 7 on: each one a page
/// loads is in its capture's record (`cache`'s), so changing one draws again
/// only the pages that load it. Part of every capture's cache key, so
/// **bump it** when any of those changes what a page's frames look like or
/// what is said about them, and every capture is redone. 2: layout notes
/// (#813), which a capture kept from before them would never say. 3: no
/// WebSocket or WebRTC traffic, and a note when a page tries (#839), so a
/// page that drew what the network told it is drawn again without it. 4:
/// `scorsese.clips` (#810), so a page that read it before it was there, and
/// drew its error, is drawn again with it; and a slot's captures now sit on
/// shelves, which a capture kept from before them is not on. 5: tiles drawn
/// on the CPU (`--disable-gpu-rasterization`, #874), which moves a page's
/// pixels a little and makes an animated blur the same every capture. 6:
/// `scorsese.words` (#811), for the same reason `clips` was 4. 7: lottie-web
/// shipped (#903), and every shipped file a page loads recorded with its
/// capture — a capture from before holds no such record, so a page refused
/// `lottie.min.js` then would otherwise keep its empty frames. 8: a changed
/// tile drawn again whole (`--disable-partial-raster`, #912), which moves
/// anti-aliased edges beside an animation by a few levels and makes a frame
/// depend on its time alone, never on the frames before it.
pub const PAGE_VERSION: u32 = 8;

/// A capture, ready to be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    /// The lossless video with alpha holding the page's frames.
    pub file: PathBuf,
    /// Which of the page's frames is the file's first: zero for a capture of
    /// the whole page, and where a piece begins otherwise ([`capture_frames`]).
    /// Frame `k` of the page is frame `k - first` of the file, stamped as
    /// though the file began at zero.
    pub first: u64,
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
    cache::fresh(&slot, project_root, request).map(|(file, record)| Captured {
        file,
        first: 0,
        warnings: record.warnings,
    })
}

/// [`cached`], for a capture holding at least `frames` of the page: the whole
/// page's, or a piece [`capture_frames`] made.
pub fn cached_frames(
    project_root: &Path,
    request: &Request,
    chrome_version: &str,
    frames: Range<u64>,
) -> Option<Captured> {
    let slot = cache::slot(project_root, request, chrome_version);
    let wanted = within(request, frames);
    cache::holding(&slot, project_root, request, &wanted).map(|(file, first, record)| Captured {
        file,
        first,
        warnings: record.warnings,
    })
}

/// `frames`, inside the page's own.
fn within(request: &Request, frames: Range<u64>) -> Range<u64> {
    let end = frames.end.min(request.frames());
    frames.start.min(end)..end
}

/// One capture a render needs: the page as [`Request`] asks for it, and the
/// stretch of its frames a clip shows — what to hand [`capture_frames`] ahead
/// of the render, so that [`cached_frames`] finds it when the render looks.
#[derive(Debug, Clone, PartialEq)]
pub struct Wanted {
    /// The page, its raster, rate and clock.
    pub request: Request,
    /// The frames shown, inside the page's own: all of them for a page whose
    /// place is a group's or a matte's to decide.
    pub frames: Range<u64>,
}

impl Wanted {
    /// `frames` of `request`, cut to the page's own.
    pub fn new(request: Request, frames: Range<u64>) -> Self {
        let frames = within(&request, frames);
        Self { request, frames }
    }
}

/// The page's frames for `request`, from the cache when they are there and
/// fresh, and captured with `chrome` when they are not.
///
/// The page is served its project's files and nothing else: a link inside the
/// project that leads out of it is refused. [`capture_following`] is the same
/// with somewhere such a link may lead.
pub fn capture(
    chrome: &Chrome,
    tools: &Tools,
    project_root: &Path,
    request: &Request,
) -> Result<Captured, PageError> {
    capture_following(chrome, tools, project_root, &[], request)
}

/// [`capture`], serving the page a file its project links to when the link
/// really leads under one of the folders `follow` names.
///
/// For a caller that lays a project out with links to files kept elsewhere:
/// the web app links each media file into the owner's library, and names that
/// library here (#857). Where a link really leads is what is checked, so one
/// from a named folder on to anywhere else is still refused.
///
/// One browser, drawing from the first frame: the web app captures in a
/// container sized for exactly one (#773). [`capture_frames`] is the same with
/// a stretch and a number of browsers to choose.
pub fn capture_following(
    chrome: &Chrome,
    tools: &Tools,
    project_root: &Path,
    follow: &[PathBuf],
    request: &Request,
) -> Result<Captured, PageError> {
    let whole = 0..request.frames();
    capture_frames(chrome, tools, project_root, follow, request, whole, 1)
}

/// How many browsers a capture on this machine is worth drawing with at once —
/// what a caller with the machine to itself hands [`capture_frames`].
pub fn browsers() -> usize {
    pieces::ways()
}

/// [`capture_following`], for only `frames` of the page — the frames a still
/// or a clip shows, rather than every one from the first.
///
/// Reuses any fresh capture holding them all. Otherwise it captures from the
/// first of them, back to a whole millisecond, to the last, and keeps that as a
/// piece of the slot — or as the whole capture, when that is what was asked. A
/// long stretch is drawn by up to `browsers` at once, each its own process
/// tree of a few hundred megabytes ([`browsers`] says what a machine is worth).
pub fn capture_frames(
    chrome: &Chrome,
    tools: &Tools,
    project_root: &Path,
    follow: &[PathBuf],
    request: &Request,
    frames: Range<u64>,
    browsers: usize,
) -> Result<Captured, PageError> {
    let wanted = within(request, frames);
    if let Some(captured) = cached_frames(project_root, request, chrome.version(), wanted.clone()) {
        return Ok(captured);
    }
    let piece = request.piece_start(wanted.start)..wanted.end;
    let slot = cache::slot(project_root, request, chrome.version());
    std::fs::create_dir_all(&slot)?;
    let fonts = cache::fonts(project_root)?;
    // Written beside and moved into place, so an interrupted capture never
    // leaves frames that look finished — under a name of this process's own, so
    // two renders of one project capturing the same page cannot interleave.
    // Which shelf it goes on is known only once the page has said what it read.
    let partial = slot.join(format!(
        "part-{}-{}.{}.partial.mkv",
        piece.start,
        piece.end,
        std::process::id()
    ));
    let heard = pieces::capture_in(
        chrome,
        tools,
        capture::Served {
            project_root,
            follow,
            fonts: &fonts,
        },
        request,
        pieces::split(request, piece.clone(), browsers),
        &partial,
    )?;
    let told = told::Told::of(request, &heard.read);
    let shelf = cache::shelf(&slot, &told);
    std::fs::create_dir_all(&shelf)?;
    let (file, record_file) = cache::files(&shelf, &piece, request);
    std::fs::rename(&partial, &file)?;
    let record = cache::Record {
        loaded: heard.loaded,
        warnings: heard.warnings,
        told,
    };
    cache::keep(&record_file, &record)?;
    cache::prune(&shelf, project_root, request, &piece);
    cache::retire(&slot, &shelf);
    Ok(Captured {
        file,
        first: piece.start,
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
