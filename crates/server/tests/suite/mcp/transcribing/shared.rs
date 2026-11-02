//! One member's share of the one transcription slot (#1651): a language
//! Whisper knows, a few waiting at most, and a stopped hearing holding the
//! slot until Whisper has let it go.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use scorsese_server::jobs::{self, State};
use scorsese_server::library::transcribe::Payload;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::stopped::{imported, running};
use crate::mcp::vendors::{Mock, serve, settled};
use crate::mcp::{call, member};

fn ask(item: i64, language: &str) -> Payload {
    Payload {
        item,
        language: Some(language.to_owned()),
    }
}

/// A code of the right shape that Whisper does not know is refused before
/// anything is queued, by the tool and by the library alike.
#[sqlx::test]
async fn a_language_whisper_does_not_know_is_refused(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (id, asset, item) = imported(&pool, &state, ana, address, &who).await;
    let (said, refused) = call(
        address,
        &who,
        "transcribe",
        json!({ "project": id, "asset": asset, "language": "xx" }),
    )
    .await;
    assert!(refused, "{said}");
    let error = state
        .library
        .transcribe(ana, &ask(item, "xx"))
        .await
        .expect_err("refused");
    assert!(error.to_string().contains("like `pt` or `en`"), "{error}");
    let all = jobs::store::list(&state.pool, ana).await.expect("listed");
    assert!(all.iter().all(|job| job.kind != "transcribe"), "{all:?}");
}

/// Behind the one running, a member may have three waiting; a fourth is
/// refused with a sentence, while the same one asked again is still handed
/// back and another member still queues.
#[sqlx::test]
async fn a_fourth_waiting_transcription_is_refused(pool: PgPool) {
    let mock = Mock {
        hang: true,
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (_, _, item) = imported(&pool, &state, ana, address, &who).await;
    let first = state.library.transcribe(ana, &ask(item, "pt")).await;
    let first = first.expect("queued");
    running(&state, ana).await;
    let mut queued = vec![first.id];
    for language in ["en", "es", "fr"] {
        let job = state.library.transcribe(ana, &ask(item, language)).await;
        queued.push(job.expect("queued").id);
    }
    let error = state
        .library
        .transcribe(ana, &ask(item, "de"))
        .await
        .expect_err("a fourth waiting");
    assert!(error.to_string().contains("waiting"), "{error}");
    let again = state.library.transcribe(ana, &ask(item, "en")).await;
    assert_eq!(again.expect("handed back").id, queued[1]);

    let (bea, bea_who) = member(&pool, "bea@example.com").await;
    let (_, _, theirs) = imported(&pool, &state, bea, address, &bea_who).await;
    let theirs = state.library.transcribe(bea, &ask(theirs, "de")).await;
    let theirs = theirs.expect("another member queues");
    call(address, &bea_who, "jobs", json!({ "cancel": theirs.id })).await;
    for job in queued.iter().rev() {
        call(address, &who, "jobs", json!({ "cancel": job })).await;
    }
    assert_eq!(settled(&state, ana).await, vec!["cancelled"; 4]);
}

/// Stopped, a transcription shows stopped at once; the next one starts only
/// once the Whisper it asked has let the request go.
#[sqlx::test]
async fn a_stopped_transcription_holds_the_slot_until_whisper_lets_go(pool: PgPool) {
    let late = Arc::new(AtomicBool::new(false));
    let mock = Mock {
        hang: true,
        late: Some(Arc::clone(&late)),
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (_, _, item) = imported(&pool, &state, ana, address, &who).await;
    let first = state.library.transcribe(ana, &ask(item, "pt")).await;
    let first = first.expect("queued");
    running(&state, ana).await;
    let next = state.library.transcribe(ana, &ask(item, "en")).await;
    let next = next.expect("queued");

    let stopped = Instant::now();
    call(address, &who, "jobs", json!({ "cancel": first.id })).await;
    let state_of = |id: i64| {
        let pool = state.pool.clone();
        async move {
            jobs::store::get(&pool, ana, id)
                .await
                .expect("read")
                .expect("held")
                .state
        }
    };
    while state_of(first.id).await != State::Cancelled {
        assert!(
            stopped.elapsed() < Duration::from_secs(5),
            "not shown stopped at once"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(state_of(next.id).await, State::Waiting, "started too soon");
    assert_eq!(mock.heard.load(Ordering::SeqCst), 1);

    late.store(true, Ordering::SeqCst);
    running(&state, ana).await;
    assert_eq!(state_of(next.id).await, State::Running);
    call(address, &who, "jobs", json!({ "cancel": next.id })).await;
    assert_eq!(settled(&state, ana).await, vec!["cancelled"; 2]);
}
