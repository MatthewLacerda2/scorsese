//! Keeping the render cache near its quota: the rule in [`super`], and the
//! weekly sweep that applies it whatever the pressure.
//!
//! These are the render cache's privileged queries — cross-user by nature,
//! since the quota is the whole machine's and the idle rule is everybody's.
//! A user's own requests never reach them.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::{IDLE, RenderCache};
use crate::db;

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
    /// Bytes they held.
    pub bytes: u64,
}

/// Make room for a new render of `size` bytes, if the quota asks for it, and
/// run `admit` — which moves the file into place and writes its row — with
/// nothing else counting or sweeping the cache meanwhile.
///
/// Over the quota, every render idle longer than [`IDLE`] goes. Still over it
/// afterwards, the new render is admitted anyway and a warning is logged: the
/// quota is a target, not a wall.
pub async fn admit<T>(
    pool: &PgPool,
    cache: &RenderCache,
    size: u64,
    admit: impl Future<Output = T>,
) -> Result<T, sqlx::Error> {
    let _admitting = cache.admitting.lock().await;
    let total = total(pool).await?;
    let quota = cache.quota().get();
    if total.saturating_add(size) > quota {
        let evicted = idle(pool, cache).await?;
        let after = total.saturating_sub(evicted.bytes).saturating_add(size);
        if after > quota {
            eprintln!(
                "scorsese-server: warning: the render cache holds {after} bytes, over its \
                 quota of {quota}, and nothing in it has been idle for {} hours; \
                 keeping the new render anyway",
                IDLE.as_secs() / 3600
            );
        }
    }
    Ok(admit.await)
}

/// The weekly sweep: every idle render, and every file no row names any more
/// — a deleted project's or account's.
pub async fn sweep(pool: &PgPool, cache: &RenderCache) -> Result<Evicted, sqlx::Error> {
    let _admitting = cache.admitting.lock().await;
    let mut evicted = idle(pool, cache).await?;
    let orphans = orphans(pool, cache).await?;
    evicted.renders += orphans.renders;
    evicted.bytes += orphans.bytes;
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
                        "scorsese-server: swept the render cache: {} renders, {} bytes",
                        evicted.renders, evicted.bytes
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

/// Delete every render idle longer than [`IDLE`] that nobody has open.
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
