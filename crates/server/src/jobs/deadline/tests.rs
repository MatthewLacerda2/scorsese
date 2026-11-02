//! [`super::kept`], alone and in the worker.

use std::convert::Infallible;

use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::*;
use crate::events::Events;
use crate::jobs::{Context, Job, Kind, Queue, Registry, State, work};

const SHORT: Allowance = Allowance {
    base: Duration::from_millis(50),
    per_second: Duration::from_millis(100),
};

/// A job that stops only when its cancel is tripped, as a render does.
async fn stops_when_told(cancel: Cancel) -> Result<Outcome, Infallible> {
    while !cancel.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    Ok(Outcome::Cancelled("stopped".into()))
}

#[tokio::test]
async fn a_job_past_its_deadline_is_stopped_and_fails_in_words() {
    let cancel = Cancel::new();
    let started = Instant::now();
    let outcome = kept(
        stops_when_told(cancel.clone()),
        SHORT,
        &Work::default(),
        &cancel,
        "render",
    )
    .await;
    assert!(cancel.is_cancelled());
    assert!(started.elapsed() >= SHORT.base);
    assert_eq!(
        outcome,
        Ok(Outcome::Failed(
            "this render took longer than scorsese allows (1 minute) and was stopped".into()
        ))
    );
}

#[tokio::test]
async fn a_job_that_finishes_as_its_deadline_comes_keeps_what_it_made() {
    let cancel = Cancel::new();
    let finishing = {
        let cancel = cancel.clone();
        async move {
            while !cancel.is_cancelled() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Ok::<_, Infallible>(Outcome::Done(serde_json::json!("made")))
        }
    };
    let outcome = kept(finishing, SHORT, &Work::default(), &cancel, "render").await;
    assert_eq!(outcome, Ok(Outcome::Done(serde_json::json!("made"))));
}

#[tokio::test]
async fn a_job_inside_its_deadline_is_left_alone() {
    let cancel = Cancel::new();
    let done = async { Ok::<_, Infallible>(Outcome::Done(serde_json::json!(1))) };
    let outcome = kept(done, SHORT, &Work::default(), &cancel, "render").await;
    assert_eq!(outcome, Ok(Outcome::Done(serde_json::json!(1))));
    assert!(!cancel.is_cancelled());
}

#[tokio::test]
async fn the_deadline_grows_with_the_work_measured() {
    let cancel = Cancel::new();
    let work = Work::default();
    let measured = work.clone();
    let job = {
        let cancel = cancel.clone();
        async move {
            // Two seconds of work: a quarter of a second in all.
            measured.set(2.0);
            stops_when_told(cancel).await
        }
    };
    let started = Instant::now();
    let outcome = kept(job, SHORT, &work, &cancel, "transcription").await;
    assert!(
        started.elapsed() >= Duration::from_millis(250),
        "{:?}",
        started.elapsed()
    );
    assert!(matches!(outcome, Ok(Outcome::Failed(_))));
}

#[tokio::test]
async fn time_granted_for_captures_moves_the_deadline() {
    let cancel = Cancel::new();
    let work = Work::default();
    work.extend(Duration::from_millis(200));
    let started = Instant::now();
    let outcome = kept(
        stops_when_told(cancel.clone()),
        SHORT,
        &work,
        &cancel,
        "render",
    )
    .await;
    assert!(
        started.elapsed() >= Duration::from_millis(250),
        "{:?}",
        started.elapsed()
    );
    assert!(matches!(outcome, Ok(Outcome::Failed(_))));
}

#[test]
fn each_local_kind_is_named_in_its_owners_words() {
    use super::super::kinds::{self, called};
    for (kind, said) in [
        (kinds::RENDER, "render"),
        (kinds::PREVIEW, "preview"),
        (kinds::PROXY, "preview proxy"),
        (kinds::THUMBNAIL, "thumbnail"),
        (kinds::TRANSCRIBE, "transcription"),
    ] {
        assert_eq!(called(kind.name), said);
    }
    assert_eq!(called("veo_shot"), "job");
}

#[test]
fn the_allowance_is_a_base_and_so_much_a_second() {
    let render = Allowance {
        base: Duration::from_secs(600),
        per_second: Duration::from_secs(10),
    };
    assert_eq!(render.of(0.0), Duration::from_secs(600));
    assert_eq!(render.of(60.0), Duration::from_secs(1200));
    assert_eq!(render.of(-5.0), Duration::from_secs(600));
    assert_eq!(render.of(f64::NAN), Duration::from_secs(600));
    assert_eq!(overran("render", Duration::from_secs(1201)), {
        "this render took longer than scorsese allows (21 minutes) and was stopped"
    });
}

/// A render that never finishes by itself: it stops only when told.
async fn endless(_: Job, context: Context) -> Outcome {
    while !context.cancel().is_cancelled() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Outcome::Cancelled("stopped".into())
}

/// The kind the worker test runs.
const ECHO: Kind = Kind {
    name: "echo",
    limit: 1,
    budget: None,
};

/// The worker end to end: a job past a short deadline is stopped, failed in
/// words, and not run again by a restart.
#[sqlx::test]
async fn a_job_past_its_deadline_is_stopped_and_not_tried_again(pool: PgPool) {
    crate::db::migrate(&pool)
        .await
        .expect("the migrations apply");
    let members = crate::db::member_pool(&pool)
        .await
        .expect("members connect");
    let ana = crate::accounts::users::create(&pool, "ana@example.com")
        .await
        .expect("the account is created");
    let job = {
        let mut tx = crate::db::scoped(&members, ana).await.expect("a scope");
        let job = super::super::store::enqueue(&mut tx, ECHO, &serde_json::json!({}))
            .await
            .expect("enqueued");
        tx.commit().await.expect("committed");
        job
    };
    let queue = Queue::new(Events::new());
    let run = |registry: Registry| {
        let (stop, stopping) = watch::channel(false);
        let worker = tokio::spawn(work(members.clone(), registry, queue.clone(), stopping));
        (stop, worker)
    };
    let (stop, worker) = run(Registry::new()
        .register(ECHO, endless)
        .allowance(ECHO, SHORT));
    let mut failed = None;
    for _ in 0..200 {
        let now = super::super::store::get(&members, ana, job.id)
            .await
            .expect("reads");
        if let Some(now) = now.filter(|now| now.state == State::Failed) {
            failed = Some(now);
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    stop.send_replace(true);
    worker.await.expect("the worker stops");
    let failed = failed.expect("the job failed at its deadline");
    assert_eq!(
        failed.error.as_deref(),
        Some("this job took longer than scorsese allows (1 minute) and was stopped")
    );
    assert_eq!(failed.attempts, 1);

    let (stop, worker) = run(Registry::new().register(ECHO, endless));
    tokio::time::sleep(Duration::from_millis(300)).await;
    stop.send_replace(true);
    worker.await.expect("the worker stops");
    let after = super::super::store::get(&members, ana, job.id)
        .await
        .expect("reads");
    assert_eq!(
        after.map(|job| job.state),
        Some(State::Failed),
        "never run again"
    );
    members.close().await;
}
