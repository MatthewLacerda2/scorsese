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
