//! Enqueueing is per user; claiming is safe, fair, and keeps to its kinds.

use scorsese_server::jobs::{State, store};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{ECHO, OTHER, account, enqueue, members};

#[sqlx::test]
async fn a_job_is_its_owners_and_nobody_elses(pool: PgPool) {
    let (ana, bia) = (
        account(&pool, "ana@example.com").await,
        account(&pool, "bia@example.com").await,
    );
    let members = members(&pool).await;
    let job = enqueue(&members, ana, ECHO, json!({"n": 1})).await;
    assert_eq!((job.state, job.attempts), (State::Waiting, 0));

    assert_eq!(
        store::list(&members, ana).await.unwrap(),
        std::slice::from_ref(&job)
    );
    assert!(store::list(&members, bia).await.unwrap().is_empty());
    assert_eq!(store::get(&members, bia, job.id).await.unwrap(), None);
}

#[sqlx::test]
async fn a_claim_takes_the_oldest_waiting_job_of_its_kinds(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let other = enqueue(&members, ana, OTHER, json!({})).await;
    let first = enqueue(&members, ana, ECHO, json!({"n": 1})).await;
    let second = enqueue(&members, ana, ECHO, json!({"n": 2})).await;

    let (job, view) = store::claim(&members, &["echo"]).await.unwrap().unwrap();
    assert_eq!((job.id, job.user, job.attempts), (first.id, ana, 1));
    assert_eq!(job.payload, json!({"n": 1}));
    assert_eq!(view.state, State::Running);

    let (job, _) = store::claim(&members, &["echo"]).await.unwrap().unwrap();
    assert_eq!(job.id, second.id);
    // Only `other` is left, and nobody asked for it.
    assert!(store::claim(&members, &["echo"]).await.unwrap().is_none());
    let (job, _) = store::claim(&members, &["other"]).await.unwrap().unwrap();
    assert_eq!(job.id, other.id);
}

#[sqlx::test]
async fn one_users_batch_does_not_hold_up_another(pool: PgPool) {
    let (ana, bia) = (
        account(&pool, "ana@example.com").await,
        account(&pool, "bia@example.com").await,
    );
    let members = members(&pool).await;
    let ana1 = enqueue(&members, ana, ECHO, json!({})).await;
    let ana2 = enqueue(&members, ana, ECHO, json!({})).await;
    let bia1 = enqueue(&members, bia, ECHO, json!({})).await;

    let mut order = Vec::new();
    while let Some((job, _)) = store::claim(&members, &["echo"]).await.unwrap() {
        order.push(job.id);
    }
    // Ana's first, then Bia's — Ana already has one running — then Ana's.
    assert_eq!(order, [ana1.id, bia1.id, ana2.id]);
}

#[sqlx::test]
async fn a_job_another_claim_holds_is_passed_over(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let held = enqueue(&members, ana, ECHO, json!({})).await;
    let free = enqueue(&members, ana, ECHO, json!({})).await;

    // Another claim, mid-transaction, has the oldest row locked.
    let mut other = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM jobs WHERE id = $1 FOR UPDATE")
        .bind(held.id)
        .execute(&mut *other)
        .await
        .unwrap();

    let (job, _) = store::claim(&members, &["echo"]).await.unwrap().unwrap();
    assert_eq!(job.id, free.id);
    other.rollback().await.unwrap();
}
