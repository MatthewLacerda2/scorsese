//! The render job: a stored project, laid out as a `.scor` folder for a
//! moment, rendered by `scorsese-render` exactly as `scorsese render` renders
//! a folder, and kept in the cache.
//!
//! The first user of the materialiser ([`crate::projects::media::materialise`]):
//! the document written, each file it uses linked by hash from the owner's
//! library, the renderer run on it unchanged. Nothing here draws a frame or
//! runs ffmpeg itself.
//!
//! **What cannot be rendered yet.** A stored project has no `recipes/` (#560),
//! so a `synth_audio` asset renders only when its bake is already in the
//! owner's library. One whose bake is not is refused **by name, recipe
//! included** — a render that silently lost its music is worse than one that
//! says why it did not happen. A file the library does not hold is refused
//! the same way.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use scorsese_core::{GenerationState, Project};
use scorsese_render::{FrameRange, RenderSettings, Renderer, Tools};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{RenderCache, RenderView, Settings, evict, store};
use crate::db::UserId;
use crate::jobs::{Context, Handler, Job, Outcome};
use crate::projects::media::{Materialised, hashes, materialise};
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
    let media = library(context, storage, job.user, &project).await?;
    let (tools, at, written) = (tools.clone(), work.0.join("project.scor"), out.clone());
    tokio::task::spawn_blocking(move || produce(&tools, &project, &at, &media, settings, &written))
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
                tx.commit().await.map_err(database)?;
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
    Ok(done(&view))
}

/// Lay the project out at `at` and render it to `out`. Blocking: a render is
/// minutes of CPU, so it runs off the server's async threads.
fn produce(
    tools: &Tools,
    project: &Project,
    at: &Path,
    media: &HashMap<String, PathBuf>,
    settings: RenderSettings,
    out: &Path,
) -> Result<(), String> {
    let laid = materialise(project, at, &|hash: &str| media.get(hash).cloned())
        .map_err(|error| format!("laying the project out: {error}"))?;
    unrenderable(project, &laid)?;
    Renderer::new(tools, settings)
        .render(project, laid.root(), FrameRange::ALL, out)
        .map_err(|error| format!("rendering: {error}"))?;
    Ok(())
}

/// Refuse, by name, every clip's asset the server cannot supply: a file the
/// library does not hold, or a synthesised sound whose bake it does not.
fn unrenderable(project: &Project, laid: &Materialised) -> Result<(), String> {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();
    for (_, clip) in project.clips() {
        let Some(asset) = project
            .asset(&clip.asset)
            .filter(|_| seen.insert(&clip.asset))
        else {
            continue; // Unknown ids are the renderer's own check to report.
        };
        let absent = laid.missing().contains(&asset.id);
        if asset.kind.is_synthesized() {
            let baked = asset.state == Some(GenerationState::Generated)
                && asset.path.is_some()
                && asset.sha256.is_some();
            if absent || !baked {
                let recipe = asset
                    .recipe
                    .as_ref()
                    .map_or("(none)".into(), ToString::to_string);
                problems.push(format!(
                    "`{}` is synthesised from the recipe {recipe}, which the server does not \
                     store yet (#560), and its bake is not in your library",
                    asset.id
                ));
            }
        } else if absent {
            problems.push(format!(
                "`{}` names a file that is not in your library",
                asset.id
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "cannot render this project: {}",
            problems.join("; ")
        ))
    }
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
    let files: Vec<(String, String)> =
        sqlx::query_as("SELECT sha256, extension FROM library_items WHERE sha256 = ANY($1)")
            .bind(&hashes)
            .fetch_all(&mut *tx)
            .await
            .map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(files
        .into_iter()
        .map(|(hash, extension)| {
            let path = storage.library_file(user, &hash, &extension);
            (hash, path)
        })
        .collect())
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
