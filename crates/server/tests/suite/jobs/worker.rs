//! The worker end to end, with handlers that stand in for real work.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use scorsese_server::events::Events;
use scorsese_server::jobs::{Context, Job, Outcome, Queue, Registry, State};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{ECHO, account, enqueue, members, start_worker, stop_worker, when};

/// Stands in for Veo: counts what it was paid for, finishes when told.
#[derive(Default)]
struct FakeVeo {
    submitted: AtomicUsize,
    ready: AtomicBool,
}

/// A shot as a real handler must run it: poll a ticket it has, submit and
/// keep the ticket only when it has none.
async fn shot(veo: Arc<FakeVeo>, job: Job, context: Context) -> Outcome {
    let ticket = match job.ticket {
        Some(ticket) => ticket,
        None => {
            veo.submitted.fetch_add(1, Ordering::SeqCst);
            let ticket = "operations/1".to_owned();
            if let Err(error) = context.keep_ticket(&ticket).await {
                return Outcome::Stuck(error.to_string());
            }
            ticket
        }
    };
    while !veo.ready.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Outcome::Done(json!({ "ticket": ticket }))
}

#[sqlx::test]
async fn a_shot_cut_off_after_submitting_is_polled_never_paid_for_twice(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let veo = Arc::new(FakeVeo::default());
    let registry = || {
        let veo = Arc::clone(&veo);
        Registry::new().register(ECHO, move |job, context| {
            shot(Arc::clone(&veo), job, context)
        })
    };
    let queue = Queue::new(Events::new());
    let job = enqueue(&members, ana, ECHO, json!({})).await;

    let first = start_worker(&members, registry(), &queue);
    for _ in 0..200 {
        let kept: Option<String> = sqlx::query_scalar("SELECT ticket FROM jobs WHERE id = $1")
            .bind(job.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        if kept.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    // The power goes, mid-generation.
    stop_worker(first).await;
    veo.ready.store(true, Ordering::SeqCst);

    let second = start_worker(&members, registry(), &queue);
    let done = when(&members, ana, job.id, State::Done).await;
    stop_worker(second).await;
    assert_eq!(veo.submitted.load(Ordering::SeqCst), 1);
    assert_eq!(done.result, Some(json!({ "ticket": "operations/1" })));
    assert_eq!(done.attempts, 2);
    assert!(done.interrupted_at.is_some());
}

#[sqlx::test]
async fn no_more_of_a_kind_run_at_once_than_its_limit(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let (now, most) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let (now_, most_) = (Arc::clone(&now), Arc::clone(&most));
    let registry = Registry::new().register(ECHO, move |_job, _context| {
        let (now, most) = (Arc::clone(&now_), Arc::clone(&most_));
        async move {
            most.fetch_max(now.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(150)).await;
            now.fetch_sub(1, Ordering::SeqCst);
            Outcome::Done(json!(null))
        }
    });
    let queue = Queue::new(Events::new());
    let mut jobs = Vec::new();
    for _ in 0..5 {
        jobs.push(enqueue(&members, ana, ECHO, json!({})).await);
    }

    let worker = start_worker(&members, registry, &queue);
    for job in &jobs {
        when(&members, ana, job.id, State::Done).await;
    }
    stop_worker(worker).await;
    assert_eq!(most.load(Ordering::SeqCst), ECHO.limit);
}

#[sqlx::test]
async fn a_handler_that_panics_fails_its_job_and_not_the_worker(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let registry = Registry::new().register(ECHO, |job: Job, _context| async move {
        assert!(job.payload["fine"].as_bool().unwrap_or(false), "a bug");
        Outcome::Done(job.payload)
    });
    let queue = Queue::new(Events::new());
    let broken = enqueue(&members, ana, ECHO, json!({ "fine": false })).await;
    let fine = enqueue(&members, ana, ECHO, json!({ "fine": true })).await;

    let worker = start_worker(&members, registry, &queue);
    let failed = when(&members, ana, broken.id, State::Failed).await;
    when(&members, ana, fine.id, State::Done).await;
    stop_worker(worker).await;
    assert!(failed.error.unwrap().contains("crashed"));
}
