//! The 48-hour rule: on quota pressure and weekly, every idle render goes —
//! except one somebody has open — and the quota is a target, not a wall.

use scorsese_server::renders::{Quota, RenderCache, evict};
use sqlx::postgres::PgPool;

use super::{card, common, idle_for, kept, member, on_disk, rows, stored};

/// A cache whose quota the renders below already fill.
fn tight(label: &str) -> RenderCache {
    RenderCache::new(common::scratch(label), Quota::bytes(60))
}

#[sqlx::test]
async fn pressure_evicts_every_idle_render_and_nothing_fresh_or_open(pool: PgPool) {
    let cache = tight("evict-pressure");
    let (ana, _) = member(&pool, "ana@example.com").await;
    let (bea, _) = member(&pool, "bea@example.com").await;
    let ana_project = stored(&pool, ana, &card(|_| {})).await;
    let bea_project = stored(&pool, bea, &card(|_| {})).await;
    let old = kept(&pool, &cache, ana, ana_project, &"1".repeat(64)).await;
    let others = kept(&pool, &cache, bea, bea_project, &"2".repeat(64)).await;
    let fresh = kept(&pool, &cache, ana, ana_project, &"3".repeat(64)).await;
    let open = kept(&pool, &cache, bea, bea_project, &"4".repeat(64)).await;
    for view in [&old, &others, &open] {
        idle_for(&pool, view.id, 49).await;
    }
    let pin = cache.pin(&RenderCache::relative(bea, bea_project, &open.key, "mp4"));

    let admitted = evict::admit(&pool, &cache, 10, async { "admitted" })
        .await
        .unwrap();

    assert_eq!(admitted, "admitted");
    assert_eq!(
        rows(&pool).await,
        2,
        "the two idle, unopened renders went — whoever's"
    );
    assert!(!on_disk(&cache, ana, &old) && !on_disk(&cache, bea, &others));
    assert!(on_disk(&cache, ana, &fresh), "used within 48 hours");
    assert!(on_disk(&cache, bea, &open), "being downloaded");
    drop(pin);
}

#[sqlx::test]
async fn under_the_quota_nothing_is_evicted_however_idle(pool: PgPool) {
    let cache = RenderCache::new(common::scratch("evict-roomy"), Quota::bytes(1_000));
    let (ana, _) = member(&pool, "ana@example.com").await;
    let project = stored(&pool, ana, &card(|_| {})).await;
    let old = kept(&pool, &cache, ana, project, &"1".repeat(64)).await;
    idle_for(&pool, old.id, 500).await;

    evict::admit(&pool, &cache, 10, async {}).await.unwrap();

    assert_eq!(rows(&pool).await, 1);
    assert!(on_disk(&cache, ana, &old));
}

#[sqlx::test]
async fn over_the_quota_with_nothing_idle_the_new_render_is_kept_anyway(pool: PgPool) {
    let cache = tight("evict-wall");
    let (ana, _) = member(&pool, "ana@example.com").await;
    let project = stored(&pool, ana, &card(|_| {})).await;
    let recent = kept(&pool, &cache, ana, project, &"1".repeat(64)).await;
    idle_for(&pool, recent.id, 47).await;

    let kept_new = evict::admit(&pool, &cache, 1_000, async { true })
        .await
        .unwrap();

    assert!(kept_new, "the new render is admitted over the quota");
    assert_eq!(rows(&pool).await, 1);
    assert!(on_disk(&cache, ana, &recent), "47 hours is not idle");
}

#[sqlx::test]
async fn the_sweep_takes_idle_renders_and_files_no_row_names(pool: PgPool) {
    let cache = RenderCache::new(common::scratch("evict-sweep"), Quota::bytes(u64::MAX));
    let (ana, _) = member(&pool, "ana@example.com").await;
    let project = stored(&pool, ana, &card(|_| {})).await;
    let doomed = stored(&pool, ana, &card(|_| {})).await;
    let old = kept(&pool, &cache, ana, project, &"1".repeat(64)).await;
    let fresh = kept(&pool, &cache, ana, project, &"2".repeat(64)).await;
    let orphan = kept(&pool, &cache, ana, doomed, &"3".repeat(64)).await;
    let open_orphan = kept(&pool, &cache, ana, doomed, &"4".repeat(64)).await;
    idle_for(&pool, old.id, 49).await;
    let pin = cache.pin(&RenderCache::relative(ana, doomed, &open_orphan.key, "mp4"));
    // A deleted project takes its rows with it, and leaves its files behind.
    scorsese_server::projects::delete(&pool, ana, doomed)
        .await
        .unwrap();

    let swept = evict::sweep(&pool, &cache).await.unwrap();

    assert_eq!(swept.renders, 2, "{swept:?}");
    assert!(!on_disk(&cache, ana, &old) && !on_disk(&cache, ana, &orphan));
    assert!(on_disk(&cache, ana, &fresh));
    assert!(on_disk(&cache, ana, &open_orphan), "open, so not yet");
    drop(pin);
    assert_eq!(evict::sweep(&pool, &cache).await.unwrap().renders, 1);
    assert!(!on_disk(&cache, ana, &open_orphan));
}

/// A capture in `user`'s page cache for `project`, last used `hours` ago:
/// 50 bytes of frames. The project's folder.
fn captured(
    cache: &RenderCache,
    user: scorsese_server::db::UserId,
    project: i64,
    hours: u64,
) -> std::path::PathBuf {
    let folder = cache.captures().pages(user, project);
    let slot = folder.join("pages/0123");
    let setup = "the test setup works";
    std::fs::create_dir_all(&slot).expect(setup);
    std::fs::write(slot.join("frames.mkv"), [0; 50]).expect(setup);
    let at = std::time::SystemTime::now() - std::time::Duration::from_secs(hours * 3600);
    let frames = std::fs::File::options()
        .append(true)
        .open(slot.join("frames.mkv"));
    frames.and_then(|file| file.set_modified(at)).expect(setup);
    folder
}

#[sqlx::test]
async fn page_captures_count_against_the_quota_and_go_when_idle(pool: PgPool) {
    let cache = RenderCache::new(common::scratch("evict-pages"), Quota::bytes(100));
    let (ana, _) = member(&pool, "ana@example.com").await;
    let project = stored(&pool, ana, &card(|_| {})).await;
    let other = stored(&pool, ana, &card(|_| {})).await;
    let fresh = captured(&cache, ana, project, 1);
    let idle = captured(&cache, ana, other, 49);
    let render = kept(&pool, &cache, ana, project, &"1".repeat(64)).await;

    // 50 + 50 bytes of captures and a render: over 100 only with the captures.
    evict::admit(&pool, &cache, 1, async {}).await.unwrap();

    assert!(!idle.exists(), "idle and over the quota");
    assert!(fresh.join("pages/0123/frames.mkv").is_file());
    assert!(on_disk(&cache, ana, &render), "used just now");
}

#[sqlx::test]
async fn the_sweep_takes_a_deleted_projects_captures_unless_a_job_holds_them(pool: PgPool) {
    let cache = RenderCache::new(common::scratch("evict-pages-sweep"), Quota::bytes(u64::MAX));
    let (ana, _) = member(&pool, "ana@example.com").await;
    let kept_project = stored(&pool, ana, &card(|_| {})).await;
    let doomed = stored(&pool, ana, &card(|_| {})).await;
    let alive = captured(&cache, ana, kept_project, 1);
    let orphan = captured(&cache, ana, doomed, 1);
    scorsese_server::projects::delete(&pool, ana, doomed)
        .await
        .unwrap();
    let held = evict::hold(&cache, ana, doomed).await;

    assert_eq!(evict::sweep(&pool, &cache).await.unwrap().captures, 0);
    assert!(orphan.exists(), "a job holds it");
    drop(held);
    let swept = evict::sweep(&pool, &cache).await.unwrap();

    assert_eq!((swept.captures, swept.bytes), (1, 50), "{swept:?}");
    assert!(!orphan.exists() && alive.exists());
}
