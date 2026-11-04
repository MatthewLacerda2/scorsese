//! A run's captures, held until it is over (#1085).

use std::ops::Range;
use std::path::Path;
use std::sync::atomic::AtomicU64;

use super::browser::Chrome;
use super::budget::Browsers;
use super::request::Request;
use super::{
    Captured, Laid, PageError, cache, cached_frames, capture, piece, pieces, told, within,
};
use crate::cancel::Cancel;
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
    /// once it is over: the project's root and how its caller laid it out
    /// ([`Laid`]) go together, and so do the browsers it may draw with —
    /// leased from a budget the captures drawing beside it share (#1095) — the
    /// count each frame drawn is added to (#1055) and the cancel that stops it
    /// before its next frame (#1088).
    ///
    /// A capture that stops, for that or any other reason, leaves `cache/` as
    /// it found it: its scratch folder is removed, and the slot with it if that
    /// left it empty.
    pub fn capture_frames(
        &mut self,
        chrome: &Chrome,
        tools: &Tools,
        (project_root, laid): (&Path, Laid<'_>),
        request: &Request,
        frames: Range<u64>,
        (browsers, drawn, cancel): (&Browsers, &AtomicU64, &Cancel),
    ) -> Result<Captured, PageError> {
        let wanted = within(request, frames);
        if let Some(captured) =
            self.cached_frames(project_root, request, chrome.version(), wanted.clone())
        {
            return Ok(captured);
        }
        let piece = (piece(request, wanted), 0);
        let to = (browsers, drawn, cancel);
        self.capture_piece(chrome, tools, (project_root, laid), request, piece, to)
    }

    /// [`super::capture_part`], held for this run: part `index` of `parts`
    /// captured unless a fresh capture of it is already there, then every
    /// part joined, once each is, into the capture of them all.
    pub fn capture_part(
        &mut self,
        chrome: &Chrome,
        tools: &Tools,
        (project_root, laid): (&Path, Laid<'_>),
        request: &Request,
        (parts, index): (&[Range<u64>], usize),
        (drawn, cancel): (&AtomicU64, &Cancel),
    ) -> Result<Option<Captured>, PageError> {
        let (Some(first), Some(last), Some(part)) = (parts.first(), parts.last(), parts.get(index))
        else {
            return Ok(None);
        };
        let span = first.start..last.end;
        let version = chrome.version();
        if let Some(captured) = self.cached_frames(project_root, request, version, span.clone()) {
            return Ok(Some(captured));
        }
        let slot = cache::slot(project_root, request, version);
        if cache::exactly(&slot, project_root, request, part).is_none() {
            // The first part measures its layout from the page's first frame,
            // as one capture of them all would; a later one from its own.
            let measured = if index == 0 { 0 } else { part.start };
            let one = Browsers::new(1);
            let to = (&one, drawn, cancel);
            let piece = (part.clone(), measured);
            self.capture_piece(chrome, tools, (project_root, laid), request, piece, to)?;
        }
        // A stretch of one part is that part's capture: nothing to join.
        if let Some(captured) = self.cached_frames(project_root, request, version, span) {
            return Ok(Some(captured));
        }
        self.join(tools, (project_root, laid.cache), request, version, parts)
    }

    /// The capture of every one of `parts` together, joined from a fresh
    /// capture of each and kept as the capture of them all — or `None` while
    /// one is missing or gone stale.
    fn join(
        &mut self,
        tools: &Tools,
        (project_root, laid_cache): (&Path, bool),
        request: &Request,
        version: &str,
        parts: &[Range<u64>],
    ) -> Result<Option<Captured>, PageError> {
        let slot = cache::slot(project_root, request, version);
        let mut held = Vec::new();
        for part in parts {
            let Some((file, record)) = cache::exactly(&slot, project_root, request, part) else {
                return Ok(None);
            };
            // Held, so no tidy drops it while it is joined.
            if !self.held.hold(&file) {
                return Ok(None);
            }
            held.push((file, record));
        }
        let (Some(first), Some(last)) = (parts.first(), parts.last()) else {
            return Ok(None);
        };
        let span = first.start..last.end;
        let heard = pieces::merge(
            held.iter()
                .map(|(_, record)| capture::Heard {
                    loaded: record.loaded.clone(),
                    warnings: record.warnings.clone(),
                    read: record.told.read(),
                })
                .collect(),
        );
        let told = told::Told::of(request, &heard.read);
        let shelf = cache::shelf(&slot, &told);
        let joining = scorsese_core::write::fresh_dir_beside(
            &slot.join(format!("part-{}-{}", span.start, span.end)),
            "joining",
        )?;
        let partial = joining.join("frames.mkv");
        let files: Vec<&Path> = held.iter().map(|(file, _)| file.as_path()).collect();
        let placed = pieces::join(tools, request, parts, &files, &partial).and_then(|()| {
            let (file, record_file) = cache::files(&shelf, &span, request);
            let record = cache::Record {
                loaded: heard.loaded,
                warnings: heard.warnings,
                told,
            };
            let place = cache::Place {
                project_root,
                laid_cache,
                slot: &slot,
                shelf: &shelf,
                files: (&file, &record_file),
                record: &record,
            };
            self.held.place(&partial, place, request)?;
            Ok(Captured {
                file,
                first: span.start,
                warnings: record.warnings,
            })
        });
        let _ = std::fs::remove_dir_all(&joining);
        placed.map(Some)
    }

    /// Captures `piece` of `request`, its layout measured from frame
    /// `measured` on, and keeps it in the cache: the capture
    /// [`Self::capture_frames`] and [`Self::capture_part`] make.
    fn capture_piece(
        &mut self,
        chrome: &Chrome,
        tools: &Tools,
        (project_root, laid): (&Path, Laid<'_>),
        request: &Request,
        (piece, measured): (Range<u64>, u64),
        (browsers, drawn, cancel): (&Browsers, &AtomicU64, &Cancel),
    ) -> Result<Captured, PageError> {
        // Only a capture that draws waits for browsers: as many as its length
        // is worth, or fewer when the others drawing beside it hold the rest.
        let lease = browsers.take(pieces::split(request, piece.clone(), browsers.most()).len());
        // Before a browser is started — launching one is a second or so, per
        // clip — and after the wait, which a cancel may have come during.
        if cancel.is_cancelled() {
            return Err(PageError::Cancelled);
        }
        let slot = cache::slot(project_root, request, chrome.version());
        cache::made(project_root, laid.cache, &slot)?;
        let fonts = cache::fonts(project_root, laid.cache)?;
        // Written beside and moved into place, so an interrupted capture never
        // leaves frames that look finished — inside a folder this capture
        // made, so two renders of one project capturing the same page cannot
        // interleave, whether two processes or two threads of one, and so
        // nothing placed in the cache beforehand (a link, #1672) is where
        // ffmpeg writes. Which shelf it goes on is known only once the page
        // has said what it read.
        let capturing = scorsese_core::write::fresh_dir_beside(
            &slot.join(format!("part-{}-{}", piece.start, piece.end)),
            "capturing",
        )?;
        let partial = capturing.join("frames.mkv");
        let served = capture::Served {
            project_root,
            follow: laid.follow,
            fonts: &fonts,
        };
        let pieces = pieces::split(request, piece.clone(), lease.count());
        let placed = pieces::capture_in(
            chrome,
            tools,
            served,
            request,
            (pieces, measured),
            (&partial, drawn, cancel),
        )
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
                laid_cache: laid.cache,
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
        // Placed, the frames have left it; stopped, they are hundreds of
        // megabytes nothing would ever read or tidy. Either way the folder
        // and whatever pieces are still in it go.
        let _ = std::fs::remove_dir_all(&capturing);
        if placed.is_err() {
            // Only succeeds on an empty folder: a slot this capture made and
            // nothing else has used.
            let _ = std::fs::remove_dir(&slot);
        }
        placed
    }
}
