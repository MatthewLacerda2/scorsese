//! A run's captures, held until it is over (#1085).

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

use super::browser::Chrome;
use super::request::Request;
use super::{Captured, PageError, cache, cached_frames, capture, piece, pieces, told, within};
use crate::tools::Tools;

/// The captures one run — a render, a still — has been handed, kept from the
/// cache's tidy until the run is over; and that tidy, of every shelf the run
/// captured onto, when it is dropped.
///
/// A run that captures several clips and decodes them afterwards takes them
/// all through one of these, so capturing the next clip never deletes the
/// file an earlier one was handed — nor does another run in this process
/// tidying at the same time. [`super::capture_frames`] is a run of one
/// capture: it tidies as it returns.
#[derive(Debug, Default)]
pub struct Holding {
    held: cache::Held,
}

impl Holding {
    /// [`super::cached_frames`], held for this run.
    pub fn cached_frames(
        &mut self,
        project_root: &Path,
        request: &Request,
        chrome_version: &str,
        frames: Range<u64>,
    ) -> Option<Captured> {
        // A file found can be tidied away by another run before it is held;
        // looked for again, it is not found, or another is.
        while let Some(captured) =
            cached_frames(project_root, request, chrome_version, frames.clone())
        {
            if self.held.hold(&captured.file) {
                return Some(captured);
            }
        }
        None
    }

    /// [`super::capture_frames`], held for this run, and tidied after only
    /// once it is over: the project's root and the folders its links may
    /// lead under go together, and so do how many browsers draw at once and
    /// the count each frame drawn is added to (#1055).
    pub fn capture_frames(
        &mut self,
        chrome: &Chrome,
        tools: &Tools,
        (project_root, follow): (&Path, &[PathBuf]),
        request: &Request,
        frames: Range<u64>,
        (browsers, drawn): (usize, &AtomicU64),
    ) -> Result<Captured, PageError> {
        let wanted = within(request, frames);
        if let Some(captured) =
            self.cached_frames(project_root, request, chrome.version(), wanted.clone())
        {
            return Ok(captured);
        }
        let piece = piece(request, wanted.clone());
        let slot = cache::slot(project_root, request, chrome.version());
        std::fs::create_dir_all(&slot)?;
        let fonts = cache::fonts(project_root)?;
        // Written beside and moved into place, so an interrupted capture never
        // leaves frames that look finished — under a name of this capture's
        // own, so two renders of one project capturing the same page cannot
        // interleave, whether two processes or two threads of one. Which shelf
        // it goes on is known only once the page has said what it read.
        let partial = cache::temporary(
            &slot.join(format!("part-{}-{}", piece.start, piece.end)),
            "partial.mkv",
        );
        let served = capture::Served {
            project_root,
            follow,
            fonts: &fonts,
        };
        let pieces = pieces::split(request, piece.clone(), browsers);
        let placed = pieces::capture_in(chrome, tools, served, request, pieces, (&partial, drawn))
            .and_then(|heard| {
                let told = told::Told::of(request, &heard.read);
                let shelf = cache::shelf(&slot, &told);
                let (file, record_file) = cache::files(&shelf, &piece, request);
                let record = cache::Record {
                    loaded: heard.loaded,
                    warnings: heard.warnings,
                    told,
                };
                let place = cache::Place {
                    project_root,
                    slot: &slot,
                    shelf: &shelf,
                    files: (&file, &record_file),
                    record: &record,
                };
                self.held.place(&partial, place, request)?;
                Ok(Captured {
                    file,
                    first: piece.start,
                    warnings: record.warnings,
                })
            });
        if placed.is_err() {
            // Hundreds of megabytes of frames nothing would ever read or tidy.
            let _ = std::fs::remove_file(&partial);
        }
        placed
    }
}
