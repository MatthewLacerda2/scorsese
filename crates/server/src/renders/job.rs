//! The render job: a stored project, laid out as a `.scor` folder for a
//! moment, rendered by `scorsese-render` exactly as `scorsese render` renders
//! a folder, and kept in the cache.
//!
//! The first user of the materialiser ([`crate::projects::media::materialise`]):
//! the document written, each file it uses linked by hash from the owner's
//! library, the renderer run on it unchanged. Nothing here draws a frame or
//! runs ffmpeg itself.
//!
//! **What a render does not do is bake.** A `synth_audio` asset renders from
//! its bake, which `synth_bake` keeps in the owner's library (#560) — exactly
//! as `scorsese render` renders a local folder's `generated/` and never bakes
//! on its own. The folder carries the project's recipes and script all the
//! same, with its pages, though the renderer reads only the pages. An asset whose bake is not in the
//! library is refused **by name, recipe included** — a render that silently
//! lost its music is worse than one that says why it did not happen. A file
//! the library does not hold is refused the same way.
//!
//! **Stopping.** The job's [`Context::cancel`] goes to the renderer, so a
//! cancel from its owner (#660) stops it within a frame, its ffmpeg children
//! reaped and its unfinished file removed — `scorsese_render::Cancel`. What
//! comes back is [`Outcome::Cancelled`], saying how far it got.
//!
//! **Progress.** Its [`Context::progress`] goes to the renderer beside the
//! cancel, and the worker tells the owner how far it has got (#698).
//!
//! **Pages** (#778). The folder carries the project's kept files, pages among
//! them. A project that shows a page is laid out in the capture spool instead
//! of the scratch folder, its pages are captured by the capture launcher
//! ([`crate::captures`]) and the render draws them from that cache, never
//! starting a browser itself. A capture that fails is not a failed render — the
//! clip shows its slug card — so what the render said comes back in the
//! result's `notes`, the reason a page was not captured among them: the
//! assistant reads those, since nothing else would tell it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use scorsese_core::Project;
use scorsese_render::{
    Cancel, FrameRange, Preview, Progress, RenderError, RenderSettings, Renderer, Tools,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{RenderCache, RenderView, Settings, evict, preview, store};
use crate::captures::dispatch::{self, Pages};
use crate::db::UserId;
use crate::jobs::{Context, Handler, Job, Outcome};
use crate::library::locate;
use crate::projects::ProjectFiles;
use crate::projects::media::{hashes, materialise};
use crate::storage::Storage;

/// What a render job carries: which project, the document as it was when the
/// render was asked for — so an edit made while it waits is not what gets
/// rendered under the old key — and the settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// The project.
    pub project: i64,
    /// Its key: the hash of `document` and `settings`.
    pub key: String,
    /// The settings, every default filled in.
    pub settings: Settings,
    /// The document, as asked for.
    pub document: Value,
}

/// The handler for [`crate::jobs::kinds::RENDER`]: render the payload's
/// document into `cache`, finding the owner's library files in `storage`.
pub fn handler(cache: RenderCache, tools: Tools, storage: Storage) -> impl Handler {
    move |job: Job, context: Context| {
        let (cache, tools, storage) = (cache.clone(), tools.clone(), storage.clone());
        async move {
            match render(&cache, &tools, &storage, &job, &context).await {
                Ok(done) => Outcome::Done(done),
                // Whatever stopped it, its owner asked it to stop first.
                Err(why) if context.cancel().is_cancelled() => Outcome::Cancelled(why),
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

async fn render(
    cache: &RenderCache,
    tools: &Tools,
    storage: &Storage,
    job: &Job,
    context: &Context,
) -> Result<Value, String> {
    let payload: Payload = serde_json::from_value(job.payload.clone())
        .map_err(|error| format!("the job does not describe a render: {error}"))?;
    let project = Project::from_json(&payload.document.to_string())
        .map_err(|error| format!("the project does not load: {error}"))?;
    let settings = payload.settings.render(&project)?;
    let quality = payload.settings.quality()?;
    if quality.is_some() && preview::superseded(context, job.user, &payload).await {
        return Ok(json!({ "superseded": true }));
    }

    // Two requests can race past the pending-job check; the second finds the
    // first's file here and renders nothing.
    let mut tx = context.scoped().await.map_err(database)?;
    let kept = store::take(&mut tx, payload.project, &payload.key).await;
    tx.commit().await.map_err(database)?;
    if let Some((view, path)) = kept.map_err(database)?
        && cache.absolute(Path::new(&path)).is_file()
    {
        return Ok(done(&view));
    }

    let work = Scratch::new(cache.work(job.id));
    let out = work
        .0
        .join(format!("render.{}", payload.settings.extension()));
    let spool = cache.captures();
    let pages = Pages {
        cache: spool.pages(job.user, payload.project),
        spool,
        job: job.id,
    };
    let _spooled = Scratch::new(pages.job_folder());
    let _held = evict::hold(cache, job.user, payload.project).await;
    let media = library(context, storage, job.user, &project).await?;
    let kept = kept_files(context, payload.project).await?;
    let previewing = match quality {
        Some(quality) => Some(
            preview::preview(context, storage, job.user, &project, quality)
                .await
                .map_err(database)?,
        ),
        None => None,
    };
    let (tools, written) = (tools.clone(), out.clone());
    let drawn = Drawn {
        settings,
        preview: previewing,
        cancel: context.cancel().clone(),
        progress: context.progress().clone(),
    };
    let places = Places {
        work: work.0.join("project.scor"),
        pages,
        kept,
    };
    let notes = tokio::task::spawn_blocking(move || {
        produce(&tools, &project, &places, &media, drawn, &written)
    })
    .await
    .map_err(|_| "the render crashed on the server; that is a bug".to_owned())??;

    let size = std::fs::metadata(&out)
        .map_err(|error| error.to_string())?
        .len();
    let relative = RenderCache::relative(
        job.user,
        payload.project,
        &payload.key,
        payload.settings.extension(),
    );
    let target = cache.absolute(&relative);
    let keep = async {
        let _pin = cache.pin(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::rename(&out, &target).map_err(|error| error.to_string())?;
        let path = relative.to_string_lossy();
        let size = i64::try_from(size).unwrap_or(i64::MAX);
        let mut tx = context.scoped().await.map_err(database)?;
        match store::insert(
            &mut tx,
            payload.project,
            &payload.key,
            &payload.settings,
            &path,
            size,
        )
        .await
        {
            Ok(view) => {
                let replaced = if quality.is_some() {
                    preview::replaced(
                        &mut tx,
                        cache,
                        payload.project,
                        &payload.settings,
                        &payload.key,
                    )
                    .await
                    .map_err(database)?
                } else {
                    Vec::new()
                };
                tx.commit().await.map_err(database)?;
                for file in replaced {
                    let _ = std::fs::remove_file(file);
                }
                Ok(view)
            }
            Err(error) => {
                let _ = std::fs::remove_file(&target);
                Err(gone_or(error))
            }
        }
    };
    let view = evict::admit(context.pool(), cache, size, keep)
        .await
        .map_err(database)??;
    let mut done = done(&view);
    done["notes"] = json!(notes);
    Ok(done)
}

/// What the file is drawn as: the settings, and — for a preview only — the
/// quality and proxies. A finished render's `preview` is `None`, so it is
/// rendered with no [`Preview`] and reads every original. `cancel` stops it,
/// and `progress` says how far it has got.
struct Drawn {
    settings: RenderSettings,
    preview: Option<Preview>,
    cancel: Cancel,
    progress: Progress,
}

/// Where a job lays its project out: the scratch folder, or — for a project
/// showing a page — the spool, where the capture launcher can see it.
struct Places {
    work: PathBuf,
    pages: Pages,
    kept: ProjectFiles,
}

/// Lay the project out, have its pages captured, and render it to `out`;
/// what the render said. Blocking: a render is minutes of CPU, and waiting
/// on captures minutes more, so it runs off the server's async threads.
fn produce(
    tools: &Tools,
    project: &Project,
    places: &Places,
    media: &HashMap<String, PathBuf>,
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
    // Asked before the project is laid out, of a folder with no word timings
    // in it — as the laid-out one has none either: the library keeps a line's
    // audio and not its timings yet (#811's web half).
    let requests = renderer
        .page_requests(project, &places.work)
        .map_err(rendering)?;
    let at = if requests.is_empty() {
        places.work.clone()
    } else {
        places.pages.folder()
    };
    let laid = materialise(project, &places.kept, &at, &|hash: &str| {
        media.get(hash).cloned()
    })
    .map_err(|error| format!("laying the project out: {error}"))?;
    super::refused::unrenderable(project, &laid)?;
    let (renderer, failed) = if requests.is_empty() {
        (renderer, HashMap::new())
    } else {
        let captured = places.pages.capture(&requests, &drawn.cancel)?;
        match captured.chrome {
            Some(chrome) => {
                evict::pages::touch(laid.root(), &requests, chrome.version());
                (renderer.with_chrome(chrome), captured.failed)
            }
            None => (renderer, captured.failed),
        }
    };
    let report = renderer
        .render(project, laid.root(), FrameRange::ALL, out)
        .map_err(rendering)?;
    Ok(dispatch::said(&report.notes, project, &failed))
}

/// Where each of the owner's library files `project` names is, by hash —
/// read from their own `library_items`, so a document naming somebody else's
/// hash finds nothing.
async fn library(
    context: &Context,
    storage: &Storage,
    user: UserId,
    project: &Project,
) -> Result<HashMap<String, PathBuf>, String> {
    let hashes: Vec<String> = hashes(project).into_iter().map(str::to_owned).collect();
    let mut tx = context.scoped().await.map_err(database)?;
    let files = locate::by_hash(&mut tx, storage, user, &hashes)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(files)
}

/// The project's kept files — its pages, recipes and script — read in the
/// job's own scope, so another user's project finds none.
async fn kept_files(context: &Context, project: i64) -> Result<ProjectFiles, String> {
    let mut tx = context.scoped().await.map_err(database)?;
    let files = crate::projects::files::read(&mut tx, project)
        .await
        .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(database)?;
    Ok(files)
}

/// What a finished render job says: which render, and where to fetch it.
fn done(view: &RenderView) -> Value {
    json!({ "render": view.id, "size": view.size, "file": view.file() })
}

/// A database failure, said to the job's owner without its detail.
fn database(error: sqlx::Error) -> String {
    eprintln!("scorsese-server: render job: {error}");
    "the server's database failed; try again".to_owned()
}

/// An insert that failed because the project is gone says so.
fn gone_or(error: sqlx::Error) -> String {
    let gone = error
        .as_database_error()
        .is_some_and(|error| error.is_foreign_key_violation());
    if gone {
        "the project was deleted before its render finished".to_owned()
    } else {
        database(error)
    }
}

/// A job's scratch folder: emptied when the job starts — a recovered job
/// begins again from nothing — and removed when it ends, however it ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(path: PathBuf) -> Self {
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
