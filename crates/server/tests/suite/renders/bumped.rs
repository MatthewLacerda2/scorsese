//! A render queued by the build before a schema bump is rendered by this one:
//! the job reads the document it carries through the migration.

use scorsese_server::jobs::State;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::job::{finished, worker};
use super::{call, card, common, member, stored};

#[sqlx::test]
async fn a_render_queued_before_the_bump_renders(pool: PgPool) {
    let files = common::files("bumped-render");
    let (address, _) = common::serve_with(pool.clone(), files.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &card(|_| {})).await;
    let path = format!("/api/projects/{id}/renders");
    let ask = json!({ "resolution": "64x36" });
    let (status, asked) = call(address, &cookie, "POST", &path, Some(ask)).await;
    assert_eq!(status, 202, "{asked}");
    let job = asked["job"]["id"].as_i64().expect("a job");

    let mut payload: Value = sqlx::query_scalar("SELECT payload FROM jobs WHERE id = $1")
        .bind(job)
        .fetch_one(&pool)
        .await
        .expect("the queued job");
    payload["document"]["schema_version"] = json!(55);
    sqlx::query("UPDATE jobs SET payload = $1 WHERE id = $2")
        .bind(&payload)
        .bind(job)
        .execute(&pool)
        .await
        .expect("the job is rewritten");

    let _worker = worker(&pool, &files).await;
    let job = finished(&pool, ana, job).await;
    assert_eq!(job.state, State::Done, "{:?}", job.error);
}
