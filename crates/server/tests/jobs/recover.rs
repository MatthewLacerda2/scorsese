//! After a crash: a job left running goes back in line, and the operator can
//! see whose it was.

use scorsese_server::jobs::{MAX_ATTEMPTS, State, store};
use scorsese_server::operator::{self, JobCommand};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{ECHO, account, enqueue, members};

#[sqlx::test]
async fn a_job_left_running_goes_back_in_line_and_says_so(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let running = enqueue(&members, ana, ECHO, json!({})).await;
    let waiting = enqueue(&members, ana, ECHO, json!({})).await;
    store::claim(&members, &["echo"]).await.unwrap().unwrap();

    let recovered = store::recover(&members).await.unwrap();
    assert_eq!(recovered.len(), 1, "{recovered:?}");
    let (whose, job) = &recovered[0];
    assert_eq!(
        (*whose, job.id, job.state),
        (ana, running.id, State::Waiting)
    );
    assert_eq!(job.attempts, 1);
    assert!(job.interrupted_at.is_some());

    let untouched = store::get(&members, ana, waiting.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(untouched.interrupted_at, None);

    // Claimed again, it is the same job, not a new one.
    let (again, _) = store::claim(&members, &["echo"]).await.unwrap().unwrap();
    assert_eq!((again.id, again.attempts), (running.id, 2));
}

#[sqlx::test]
async fn a_job_interrupted_too_often_is_given_up_on_but_a_ticket_is_kept(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let plain = enqueue(&members, ana, ECHO, json!({})).await;
    let paid = enqueue(&members, ana, ECHO, json!({})).await;
    sqlx::query(
        "UPDATE jobs SET state = 'running', attempts = $1,
                ticket = CASE WHEN id = $2 THEN 'operations/abc' END,
                ticket_at = CASE WHEN id = $2 THEN now() END",
    )
    .bind(MAX_ATTEMPTS)
    .bind(paid.id)
    .execute(&pool)
    .await
    .unwrap();

    store::recover(&members).await.unwrap();
    let plain = store::get(&members, ana, plain.id).await.unwrap().unwrap();
    assert_eq!(plain.state, State::Failed);
    let why = plain.error.unwrap();
    assert!(
        why.contains(&format!("interrupted {MAX_ATTEMPTS} times")),
        "{why}"
    );
    // Stuck, not failed: the ticket is the record that money was spent.
    let paid_view = store::get(&members, ana, paid.id).await.unwrap().unwrap();
    assert_eq!(paid_view.state, State::Stuck);
    let ticket: Option<String> = sqlx::query_scalar("SELECT ticket FROM jobs WHERE id = $1")
        .bind(paid.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ticket.as_deref(), Some("operations/abc"));
}

#[sqlx::test]
async fn the_operator_sees_whose_jobs_a_crash_cut_off(pool: PgPool) {
    let members = members(&pool).await;
    let none = operator::job(&members, JobCommand::Interrupted)
        .await
        .unwrap();
    assert_eq!(none, "no job has been interrupted");

    let ana = account(&pool, "ana@example.com").await;
    let job = enqueue(&members, ana, ECHO, json!({})).await;
    store::claim(&members, &["echo"]).await.unwrap().unwrap();
    store::recover(&members).await.unwrap();

    let output = operator::job(&members, JobCommand::Interrupted)
        .await
        .unwrap();
    let line = format!("{}\tana@example.com\techo\twaiting\tinterrupted ", job.id);
    assert!(output.starts_with(&line), "{output}");
    assert!(output.ends_with(" UTC"), "{output}");
}
