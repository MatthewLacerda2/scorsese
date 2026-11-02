//! Drawing a render job's file: lay the project out, have its pages
//! captured, and render — the blocking half of [`super::job`].

use std::path::{Path, PathBuf};

use scorsese_core::Project;
use scorsese_render::{
    Cancel, FrameRange, Preview, Progress, RenderError, RenderSettings, Renderer, Tools,
};

use super::{evict, room};
use crate::captures::dispatch::Pages;
use crate::jobs::Work;
use crate::library::locate::Located;
use crate::projects::ProjectFiles;
use crate::projects::media::materialise;

/// What the file is drawn as: the settings, and — for a preview only — the
/// quality and proxies. A finished render's `preview` is `None`, so it is
/// rendered with no [`Preview`] and reads every original. `cancel` stops it,
/// and `progress` says how far it has got.
pub(super) struct Drawn {
    pub(super) settings: RenderSettings,
    /// The frames asked for: the whole timeline unless a range was (#1070).
    pub(super) range: FrameRange,
    pub(super) preview: Option<Preview>,
    pub(super) cancel: Cancel,
    pub(super) progress: Progress,
    /// What the job's deadline reads (#1634): the captures it waits on are
    /// granted on top, since the launcher bounds each of them itself.
    pub(super) work: Work,
}

/// Where a job lays its project out: the scratch folder, or — for a project
/// showing a page — the spool, where the capture launcher can see it.
pub(super) struct Places {
    pub(super) work: PathBuf,
    pub(super) pages: Pages,
    pub(super) kept: ProjectFiles,
    /// The bytes its captures may write ([`room::spare`]).
    pub(super) spare: u64,
}

/// Lay the project out, have its pages captured, and render it to `out`;
/// what the render said ([`super::said`]). Blocking: a render is minutes of CPU, and waiting
/// on captures minutes more, so it runs off the server's async threads.
pub(super) fn produce(
    tools: &Tools,
    project: &Project,
    places: &Places,
    media: &Located,
    drawn: Drawn,
    out: &Path,
) -> Result<Vec<String>, String> {
    let rendering = |error: RenderError| match error {
        // Already a whole sentence: how far it got, and that nothing was kept.
        RenderError::Cancelled { .. } => error.to_string(),
        other => format!("rendering: {other}"),
    };
    // Never a browser in this container: pages come from the capture
    // container's cache, or show their cards.
    let renderer = Renderer::new(tools, drawn.settings)
        .with_cancel(drawn.cancel.clone())
        .with_progress(drawn.progress)
        .without_capturing();
    let renderer = match drawn.preview {
        Some(preview) => renderer.with_preview(preview),
        None => renderer,
    };
    // Whether it shows a page decides where it is laid out; what each page is
    // told is asked of the laid-out folder, which holds the word timings the
    // render will read too (#886).
    let shows_pages = !renderer
        .page_captures(project, &places.work)
        .map_err(rendering)?
        .is_empty();
    let at = if shows_pages {
        places.pages.folder()
    } else {
        places.work.clone()
    };
    let laid = materialise(project, &places.kept, &at, media)
        .map_err(|error| format!("laying the project out: {error}"))?;
    super::refused::unrenderable(project, &laid)?;
    let requests = if shows_pages {
        renderer
            .page_captures(project, laid.root())
            .map_err(rendering)?
    } else {
        Vec::new()
    };
    // A capture too large for the disk is never asked for (#1647).
    let (requests, mut failed) = room::capturable(requests, laid.root(), places.spare);
    let renderer = if requests.is_empty() {
        renderer
    } else {
        // The render must not run out of time waiting for captures in time.
        drawn.work.extend(crate::captures::bound(&requests));
        let captured = places.pages.capture(&requests, &drawn.cancel)?;
        failed.extend(captured.failed);
        match captured.chrome {
            Some(chrome) => {
                evict::pages::touch(laid.root(), &requests, chrome.version());
                renderer.with_chrome(chrome)
            }
            None => renderer,
        }
    };
    let report = renderer
        .render(project, laid.root(), drawn.range, out)
        .map_err(rendering)?;
    Ok(super::said::said(report, project, laid.root(), &failed))
}
