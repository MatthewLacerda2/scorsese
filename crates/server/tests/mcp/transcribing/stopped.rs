//! A transcription stopped, or asked twice (#1549).

use std::time::{Duration, Instant};

use scorsese_server::jobs::{self, State};
use scorsese_server::library::transcribe::Payload;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::footage;
use crate::vendors::{Mock, serve, settled};
use crate::{call, document, member};

/// The footage imported into its project: the project, the asset and the
/// library item.
pub(super) async fn imported(
    pool: &PgPool,
    state: &scorsese_server::http::AppState,
    user: scorsese_server::db::UserId,
    address: std::net::SocketAddr,
    who: &str,
) -> (i64, String, i64) {
    let (id, sha) = footage(pool, state, user).await;
    let item: i64 = sqlx::query_scalar("SELECT id FROM library_items WHERE sha256 = $1")
        .bind(&sha)
        .fetch_one(pool)
        .await
        .expect("held");
    let (said, refused) = call(
        address,
        who,
        "import",
        json!({ "project": id, "items": [item] }),
    )
    .await;
    assert!(!refused, "{said}");
    let asset = document(pool, user, id).await.document.assets[0]
        .id
        .to_string();
    (id, asset, item)
}

/// The jobs of `user` once one is running.
async fn running(state: &scorsese_server::http::AppState, user: scorsese_server::db::UserId) {
    for _ in 0..400 {
        let all = jobs::store::list(&state.pool, user).await.expect("listed");
        if all
            .iter()
            .any(|job| job.kind == "transcribe" && job.state == State::Running)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the transcription never started");
}

/// Stopped while the Whisper listens, the job ends at once, its slot free
/// and nothing kept: the request is dropped, not left to run on.
#[sqlx::test]
async fn a_stopped_transcription_ends_its_work_and_keeps_nothing(pool: PgPool) {
    let mock = Mock {
        hang: true,
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (id, asset, item) = imported(&pool, &state, ana, address, &who).await;
    let (said, refused) = call(
        address,
        &who,
        "transcribe",
        json!({ "project": id, "asset": asset }),
    )
    .await;
    assert!(!refused, "{said}");
    running(&state, ana).await;
    let job = jobs::store::list(&state.pool, ana).await.expect("listed");
    let job = job
        .iter()
        .find(|job| job.kind == "transcribe")
        .expect("queued");
    let stopped = Instant::now();
    let (said, refused) = call(address, &who, "jobs", json!({ "cancel": job.id })).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["cancelled"]);
    assert!(
        stopped.elapsed() < Duration::from_secs(5),
        "{:?}",
        stopped.elapsed()
    );
    let kept: Option<Value> = sqlx::query_scalar("SELECT words FROM library_items WHERE id = $1")
        .bind(item)
        .fetch_one(&pool)
        .await
        .expect("the row");
    assert_eq!(kept, None);
}

/// A job already asked the same is handed back, even to two asking at once;
/// one asked in another language is a job of its own.
#[sqlx::test]
async fn the_same_hearing_is_queued_once_and_another_language_is_its_own(pool: PgPool) {
    let mock = Mock {
        hang: true,
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (_, _, item) = imported(&pool, &state, ana, address, &who).await;
    let ask = |language: Option<&str>| Payload {
        item,
        language: language.map(str::to_owned),
    };
    let (detect, again) = (ask(None), ask(None));
    let (first, second) = tokio::join!(
        state.library.transcribe(ana, &detect),
        state.library.transcribe(ana, &again),
    );
    let (first, second) = (first.expect("queued"), second.expect("queued"));
    assert_eq!(first.id, second.id, "two asking at once queue one job");
    running(&state, ana).await;
    let behind = state
        .library
        .transcribe(ana, &detect)
        .await
        .expect("queued");
    assert_eq!(behind.id, first.id, "a running one is handed back");
    let forced = state
        .library
        .transcribe(ana, &ask(Some("pt")))
        .await
        .expect("queued");
    assert_ne!(forced.id, first.id, "a forced language is not a detection");
    for job in [first.id, forced.id] {
        call(address, &who, "jobs", json!({ "cancel": job })).await;
    }
    assert_eq!(settled(&state, ana).await, vec!["cancelled", "cancelled"]);
}
