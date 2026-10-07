//! Keeping the render cache near its quota: the rule in [`super`], and the
//! weekly sweep that applies it whatever the pressure.
//!
//! These are the render cache's privileged queries — cross-user by nature,
//! since the quota is the whole machine's and the idle rule is everybody's.
//! A user's own requests never reach them.
//!
//! Page captures are held to the same rule and the same quota ([`pages`]).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::{IDLE, Pin, RenderCache};
use crate::db::{self, UserId};

pub mod pages;

/// How often the sweep is due.
pub const SWEEP_EVERY: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// How often the server looks whether the sweep is due. The sweep's own
/// clock is a file's age (see [`run`]), so a restart does not reset it.
const LOOK_EVERY: Duration = Duration::from_secs(60 * 60);

/// The file whose modification time says when the cache was last swept.
const SWEPT: &str = "renders-swept";

/// What one pass removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Evicted {
    /// Renders deleted, rows and files.
    pub renders: u64,
    /// Page captures deleted, each a slot of frames.
    pub captures: u64,
    /// Bytes they all held.
    pub bytes: u64,
}

impl Evicted {
    fn add(&mut self, other: Self) {
        self.renders += other.renders;
        self.captures += other.captures;
        self.bytes += other.bytes;
    }
}

/// Keep `user`'s page cache for `project` whole until the pin drops: a render
/// job holds it from before its pages are captured until it is rendered.
///
/// Taken under the same lock eviction holds, so a pass either finished before
/// the job began or sees the pin.
pub async fn hold(cache: &RenderCache, user: UserId, project: i64) -> Pin {
    let _admitting = cache.admitting.lock().await;
    let folder = cache.captures().pages(user, project);
    let relative = folder.strip_prefix(&cache.root).unwrap_or(&folder);
    cache.pin(relative)
}

/// Make room for a new render of `size` bytes, if the quota asks for it, and
/// run `admit` — which moves the file into place and writes its row — with
/// nothing else counting or sweeping the cache meanwhile.
///
/// The quota counts every render and every page capture. Over it, every
/// render and every capture idle longer than [`IDLE`] goes. Still over it
/// afterwards, the new render is admitted anyway and a warning is logged: the
/// quota is a target, not a wall.
pub async fn admit<T>(
    pool: &PgPool,
    cache: &RenderCache,
    size: u64,
    admit: impl Future<Output = T>,
) -> Result<T, sqlx::Error> {
    let _admitting = cache.admitting.lock().await;
    let total = total(pool).await? + pages::total(cache);
    let quota = cache.quota().get();
    if total.saturating_add(size) > quota {
        let evicted = idle(pool, cache).await?;
        let after = total.saturating_sub(evicted.bytes).saturating_add(size);
        if after > quota {
            eprintln!(
                "scorsese-server: warning: the render cache holds {after} bytes with its \
                 page captures, over its quota of {quota}, and nothing in it has been idle \
                 for {} hours; keeping the new render anyway",
                IDLE.as_secs() / 3600
            );
        }
    }
    Ok(admit.await)
}

/// The weekly sweep: every idle render and capture, and every file no row
/// names any more — a deleted project's or account's.
pub async fn sweep(pool: &PgPool, cache: &RenderCache) -> Result<Evicted, sqlx::Error> {
    let _admitting = cache.admitting.lock().await;
    let mut evicted = idle(pool, cache).await?;
    evicted.add(orphans(pool, cache).await?);
    evicted.add(orphaned_pages(pool, cache).await?);
    Ok(evicted)
}

/// Sweep whenever the last sweep is [`SWEEP_EVERY`] old, until `stop`.
///
/// "Last" is the modification time of a file in the cache root, so the clock
/// survives restarts — a home server restarted more often than weekly would
/// otherwise never sweep — and a cache that lost the file sweeps at once,
/// which is harmless.
pub async fn run(pool: PgPool, cache: RenderCache, mut stop: watch::Receiver<bool>) {
    loop {
        if due(&cache.root.join(SWEPT)) {
            match sweep(&pool, &cache).await {
                Ok(evicted) => {
                    eprintln!(
                        "scorsese-server: swept the render cache: {} renders, {} page \
                         captures, {} bytes",
                        evicted.renders, evicted.captures, evicted.bytes
                    );
                    if let Err(error) = std::fs::write(cache.root.join(SWEPT), b"") {
                        eprintln!("scorsese-server: could not mark the sweep: {error}");
                    }
                }
                Err(error) => eprintln!("scorsese-server: the render sweep failed: {error}"),
            }
        }
        tokio::select! {
            _ = stop.wait_for(|stopping| *stopping) => return,
            () = tokio::time::sleep(LOOK_EVERY) => {}
        }
    }
}

/// Whether the file at `marker` is missing or older than [`SWEEP_EVERY`].
fn due(marker: &Path) -> bool {
    let modified = std::fs::metadata(marker).and_then(|meta| meta.modified());
    modified.map_or(true, |at| {
        SystemTime::now()
            .duration_since(at)
            .is_ok_and(|age| age >= SWEEP_EVERY)
    })
}

/// Bytes every render row says it holds, everybody's.
async fn total(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let total: i64 = sqlx::query_scalar("SELECT coalesce(sum(size), 0)::bigint FROM renders")
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(u64::try_from(total).unwrap_or(0))
}

/// Delete every render and every page capture idle longer than [`IDLE`]
/// that nobody has open.
///
/// The idle test is in the `DELETE`, so a download that stamped its row
/// after the pins were read is still spared: Postgres re-reads the row it
/// waited on (see [`super`]).
async fn idle(pool: &PgPool, cache: &RenderCache) -> Result<Evicted, sqlx::Error> {
    let pinned: Vec<String> = cache
        .pinned_paths()
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let mut tx = db::privileged(pool).await?;
    let gone: Vec<(String, i64)> = sqlx::query_as(
        "DELETE FROM renders
         WHERE last_used_at < now() - make_interval(secs => $1) AND NOT (path = ANY($2))
         RETURNING path, size",
    )
    .bind(IDLE.as_secs_f64())
    .bind(&pinned)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut evicted = Evicted::default();
    for (path, size) in gone {
        remove(&cache.absolute(Path::new(&path)));
        evicted.renders += 1;
        evicted.bytes += u64::try_from(size).unwrap_or(0);
    }
    let held = pages::held(cache);
    for (_, _, folder) in pages::projects(cache) {
        if !held.contains(&folder) {
            evicted.add(pages::idle(&folder));
            pages::tidy(&folder);
        }
    }
    Ok(evicted)
}

/// Remove every page cache whose project or account no longer exists, and
/// that no job holds.
async fn orphaned_pages(pool: &PgPool, cache: &RenderCache) -> Result<Evicted, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let known: Vec<(i64, i64)> = sqlx::query_as("SELECT user_id, id FROM projects")
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    let known: HashSet<(i64, i64)> = known.into_iter().collect();
    let held = pages::held(cache);
    let mut evicted = Evicted::default();
    for (user, project, folder) in pages::projects(cache) {
        if !known.contains(&(user, project)) && !held.contains(&folder) {
            evicted.add(pages::gone(&folder));
            pages::tidy(&folder);
        }
    }
    Ok(evicted)
}

/// Remove every render file no row names and nobody has open.
async fn orphans(pool: &PgPool, cache: &RenderCache) -> Result<Evicted, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let known: Vec<String> = sqlx::query_scalar("SELECT path FROM renders")
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    let known: HashSet<PathBuf> = known.into_iter().map(PathBuf::from).collect();
    let pinned: HashSet<PathBuf> = cache.pinned_paths().into_iter().collect();
    let mut evicted = Evicted::default();
    for relative in files(&cache.root) {
        if known.contains(&relative) || pinned.contains(&relative) {
            continue;
        }
        let path = cache.absolute(&relative);
        let size = std::fs::metadata(&path).map_or(0, |meta| meta.len());
        remove(&path);
        evicted.renders += 1;
        evicted.bytes += size;
        if let Some(project) = path.parent() {
            // Only succeeds once the folder is empty, which is the point.
            let _ = std::fs::remove_dir(project);
        }
    }
    Ok(evicted)
}

/// Every file at `users/*/renders/*/*` under `root`, relative to it.
fn files(root: &Path) -> Vec<PathBuf> {
    let entries = |path: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(path)
            .map(|read| read.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default()
    };
    let mut found = Vec::new();
    for user in entries(&root.join(super::USERS)) {
        for project in entries(&user.join(super::RENDERS)) {
            for file in entries(&project) {
                if let Ok(relative) = file.strip_prefix(root) {
                    found.push(relative.to_path_buf());
                }
            }
        }
    }
    found
}

/// Delete a render file. One already gone is deleted; anything else is said,
/// and the next sweep tries again.
fn remove(path: &Path) {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            eprintln!(
                "scorsese-server: could not delete {}: {error}",
                path.display()
            );
        }
        _ => {}
    }
}
