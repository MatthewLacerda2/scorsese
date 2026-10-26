//! `jobs` with `wait: true` (#1301): one call answers once the job has ended,
//! rather than a client asking over and over.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::{fund, quote_and_confirm};
use super::vendors::{Mock, serve};
use super::{call, common, member, stored};

/// The job id in an answer such as `hero: queued as job 4, …`.
fn job_in(said: &str) -> i64 {
    said.split_whitespace()
        .skip_while(|word| *word != "job")
        .nth(1)
        .and_then(|word| word.trim_end_matches([',', '.', ';']).parse().ok())
        .unwrap_or_else(|| panic!("no job in {said}"))
}

#[sqlx::test]
async fn a_queued_shot_is_waited_on_in_one_call(pool: PgPool) {
    let mock = Mock::default();
    let (address, _state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let shot = json!({ "assets": [
        { "id": "hero", "kind": "generated_video", "state": "sketch",
          "prompt": "a lighthouse at dawn" }
    ] });
    let id = stored(&pool, ana, shot).await;
    fund(&pool, ana, 10).await;
    let (queued, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{queued}");
    let job = job_in(&queued);

    let waited = json!({ "job": job, "wait": true });
    let (said, refused) = call(address, &who, "jobs", waited.clone()).await;
    assert!(!refused, "{said}");
    assert!(
        said.starts_with(&format!("job {job} (veo_shot): done")),
        "{said}"
    );
    // Already ended: the same line, at once.
    assert_eq!(call(address, &who, "jobs", waited).await.0, said);
}

#[sqlx::test]
async fn a_waiting_render_stopped_mid_wait_answers_cancelled(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, json!({})).await;
    let (queued, _) = call(address, &who, "render", json!({ "project": id })).await;
    let job = job_in(&queued);

    // No worker here: the render waits its turn until it is cancelled.
    let waiting = {
        let who = who.clone();
        tokio::spawn(async move {
            call(address, &who, "jobs", json!({ "job": job, "wait": true })).await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!waiting.is_finished(), "answered before the job ended");
    let (stopped, refused) = call(address, &who, "jobs", json!({ "cancel": job })).await;
    assert!(!refused, "{stopped}");
    let (said, refused) = waiting.await.expect("the waiting call returns");
    assert!(
        !refused && said.contains("cancelled before it started"),
        "{said}"
    );
}

#[sqlx::test]
async fn wait_takes_one_job_and_only_the_callers(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (_, bob) = member(&pool, "bob@example.com").await;
    let id = stored(&pool, ana, json!({})).await;
    let (queued, _) = call(address, &who, "render", json!({ "project": id })).await;
    let job = job_in(&queued);

    let (said, refused) = call(address, &who, "jobs", json!({ "wait": true })).await;
    assert!(refused && said.contains("give job"), "{said}");
    let (said, refused) = call(address, &bob, "jobs", json!({ "job": job, "wait": true })).await;
    assert!(refused && said.contains(&format!("no job {job}")), "{said}");
}
