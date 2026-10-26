//! A preview that fails after its project moved on is `cancelled`, never
//! `failed` (#1439): nobody is waiting for it. Each job here fails the same
//! way — its document does not load — before anything could race, so only
//! the stored project decides how it is recorded.

use scorsese_server::db::{self, UserId};
use scorsese_server::jobs::{State, kinds, store};
use scorsese_server::projects;
use scorsese_server::renders::job::Payload;
use scorsese_server::renders::{PreviewAsk, Settings, key};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::job::finished;
use super::previews::worker;
use super::{card, common, member, stored};

/// A preview job of `project` under `key` whose document does not load.
async fn broken(pool: &PgPool, user: UserId, project: i64, key: String) -> i64 {
    let setup = "the test setup works";
    let settings = Settings::from_preview(&PreviewAsk::default()).expect(setup);
    let payload = Payload {
        project,
        key,
        settings,
        document: json!({}),
    };
    let members = db::member_pool(pool).await.expect(setup);
    let mut tx = db::scoped(&members, user).await.expect(setup);
    let job = store::enqueue(
        &mut tx,
        kinds::PREVIEW,
        &serde_json::to_value(payload).expect(setup),
    )
    .await
    .expect(setup);
    tx.commit().await.expect(setup);
    job.id
}

#[sqlx::test]
async fn a_failed_preview_is_cancelled_only_when_its_project_moved_on(pool: PgPool) {
    let files = common::files("preview-outrun");
    let (ana, _) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let document = projects::open(&pool, ana, id).await.unwrap().document;
    let settings = Settings::from_preview(&PreviewAsk::default()).unwrap();
    let current = key(&document, &settings).unwrap();

    let moved = broken(&pool, ana, id, "an older revision".to_owned()).await;
    let matching = broken(&pool, ana, id, current).await;
    let unreadable = broken(&pool, ana, id + 1000, "no such project".to_owned()).await;
    let _worker = worker(&pool, &files).await;

    for (job, state) in [
        (moved, State::Cancelled),
        (matching, State::Failed),
        // A project that cannot be read is no evidence it moved on.
        (unreadable, State::Failed),
    ] {
        let ended = finished(&pool, ana, job).await;
        assert_eq!(ended.state, state, "job {job}: {:?}", ended.error);
        assert!(ended.error.unwrap().contains("does not load"));
    }
}
