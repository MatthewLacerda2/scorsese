//! A job that fails says so in the log, inside a span naming the job, its
//! kind and its owner (#1520).

use scorsese_server::events::Events;
use scorsese_server::jobs::{Job, Outcome, Queue, Registry, State};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::common::logged;
use super::{ECHO, account, enqueue, members, start_worker, stop_worker, when};

#[sqlx::test]
async fn a_failed_job_is_logged_by_its_id_kind_and_owner(pool: PgPool) {
    failed_job_is_named(pool, None).await;
}

/// A filter keeping warnings alone still names the job a failure is in.
#[sqlx::test]
async fn a_failed_job_is_named_under_a_warn_filter(pool: PgPool) {
    failed_job_is_named(pool, Some("warn")).await;
}

async fn failed_job_is_named(pool: PgPool, filter: Option<&str>) {
    let (_logging, kept) = logged::keep_at(filter);
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let registry = Registry::new().register(ECHO, |_job: Job, _context| async move {
        Outcome::Failed("the echo would not answer".into())
    });
    let queue = Queue::new(Events::new());
    let job = enqueue(&members, ana, ECHO, json!({})).await;

    let worker = start_worker(&members, registry, &queue);
    when(&members, ana, job.id, State::Failed).await;
    stop_worker(worker).await;

    let log = kept.text();
    let named = format!("job{{id={} kind={} user={}}}", job.id, ECHO.name, ana.get());
    assert!(
        log.lines()
            .any(|line| line.contains(&named) && line.contains("failed: the echo would not answer")),
        "{named} in:\n{log}"
    );
}
