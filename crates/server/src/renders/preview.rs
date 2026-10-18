//! What a preview render does that a finished render does not (#542).
//!
//! A preview is a render — the same job, the same cache, the same key rule —
//! with its settings marked by a quality ([`super::Settings::preview`]). The
//! three differences are all here, so the render job reads as one path with
//! three questions in it:
//!
//! - **Superseded before it starts.** Edits arrive faster than renders
//!   finish, and a preview of a revision the project has moved past is a
//!   picture of a document that no longer exists. So when a preview's turn
//!   comes, it first checks the project still hashes to its key, and if not it
//!   finishes at once without rendering. Nothing cancels a waiting job; it
//!   retires itself, which needs no race with the claim. One already
//!   *running* is stopped when the next preview is queued
//!   ([`super::request::ask`], #660) — it is recorded `cancelled` — and the
//!   editor asks again for the revision it is on.
//! - **Proxies.** A reduced quality reads each heavy video's proxy where one is
//!   made, and queues the making of any that are missing
//!   ([`crate::library::proxy`]).
//! - **Pieces.** A preview played in pieces (#1448) is a render over a
//!   range, remuxed for the browser. Every piece of a cut with anything to
//!   hear carries sound, silence where its stretch has none ([`sound`]),
//!   because the browser's buffer takes the same tracks in every piece; and
//!   whether it has any is said before the first is made ([`silent`]).
//! - **One kept per shape.** A new preview replaces the project's earlier ones
//!   at the same settings: they are of documents the project has moved past.
//!   A file somebody has open is spared, exactly as eviction spares it.

use std::path::Path;

use scorsese_core::{Fps, Project};
use scorsese_render::preview::Sound;
use scorsese_render::{FrameRange, Plan, Preview, Proxies, Quality, RenderReport, SampleRate};

use super::job::Payload;
use super::{RenderCache, Settings, key};
use crate::db::{Tx, UserId};
use crate::jobs::{Context, Job};
use crate::library::proxy;
use crate::projects::{self, ProjectError};
use crate::storage::Storage;

/// Whether the project has moved on from the document this preview was asked
/// for — or is gone, which is the same answer: nobody is waiting for it.
pub(super) async fn superseded(context: &Context, user: UserId, payload: &Payload) -> bool {
    moved_on(context, user, payload).await.unwrap_or(true)
}

/// Whether the stored project no longer hashes to the preview's key, or why
/// it could not be read. A render that failed asks this rather than
/// [`superseded`]: a database hiccup must not turn a real failure into a
/// cancel (#1439).
async fn moved_on(
    context: &Context,
    user: UserId,
    payload: &Payload,
) -> Result<bool, ProjectError> {
    let stored = projects::open(context.pool(), user, payload.project).await?;
    Ok(key(&stored.document, &payload.settings).ok() != Some(payload.key.clone()))
}

/// Whether `job` is a preview the project has moved on from. A project that
/// cannot be read says no: the failure stands rather than hiding as a cancel.
pub(super) async fn outrun(job: &Job, context: &Context) -> bool {
    let Ok(payload) = serde_json::from_value::<Payload>(job.payload.clone()) else {
        return false;
    };
    matches!(payload.settings.quality(), Ok(Some(_)))
        && matches!(moved_on(context, job.user, &payload).await, Ok(true))
}

/// Whether `project` has nothing to hear: no clip on it puts sound into the
/// mix, read from what its assets table recorded, exactly as a render plans
/// its mix.
pub(crate) fn silent(project: &Project) -> bool {
    Plan::build_sound(project, project.timeline_fps, FrameRange::ALL)
        .map_or(true, |plan| plan.audio().is_empty())
}

/// The sound a piece carries, given whether its cut has any (`audible`) and
/// what its render reported: the render's mix, or silence as long as its
/// picture at the render's `rate` and `fps`.
pub(super) fn sound(audible: bool, report: &RenderReport, (rate, fps): (SampleRate, Fps)) -> Sound {
    match (audible, report.seconds_of_audio) {
        (false, _) => Sound::Absent,
        (true, Some(_)) => Sound::Mixed,
        (true, None) => {
            let samples = u128::from(report.frames) * u128::from(rate.hz()) * u128::from(fps.den())
                / u128::from(fps.num()).max(1);
            Sound::Silent(rate, u64::try_from(samples).unwrap_or(u64::MAX))
        }
    }
}

/// The preview to render `project` as at `quality`: its proxies, where the
/// quality reads them, with any that are missing queued and announced.
pub(super) async fn preview(
    context: &Context,
    storage: &Storage,
    user: UserId,
    project: &Project,
    quality: Quality,
) -> Result<Preview, sqlx::Error> {
    if !quality.uses_proxies() {
        return Ok(Preview::new(quality));
    }
    let hashes: Vec<String> = projects::media::hashes(project)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut tx = context.scoped().await?;
    let (proxies, queued): (Proxies, _) =
        proxy::for_preview(&mut tx, storage, user, &hashes).await?;
    tx.commit().await?;
    for job in &queued {
        context.queue().announce(user, job);
    }
    Ok(Preview::new(quality).with_proxies(proxies))
}

/// Forget `project`'s other previews at `settings` — the same shape at the
/// same quality, so of other documents — except any file somebody has open.
/// Hands back their files, to remove once the transaction commits.
pub(super) async fn replaced(
    tx: &mut Tx,
    cache: &RenderCache,
    project: i64,
    settings: &Settings,
    kept: &str,
) -> Result<Vec<std::path::PathBuf>, sqlx::Error> {
    let pinned: Vec<String> = cache
        .pinned_paths()
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let gone: Vec<String> = sqlx::query_scalar(
        "DELETE FROM renders
         WHERE project_id = $1 AND settings = $2 AND key <> $3 AND NOT (path = ANY($4))
         RETURNING path",
    )
    .bind(project)
    .bind(sqlx::types::Json(settings))
    .bind(kept)
    .bind(&pinned)
    .fetch_all(&mut **tx)
    .await?;
    Ok(gone
        .iter()
        .map(|path| cache.absolute(Path::new(path)))
        .collect())
}
