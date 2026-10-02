//! Stopping work over web MCP (#660): a render is a job, so stopping it is
//! `job_cancel`; a call still being served is stopped by
//! `notifications/cancelled`, through the cancel the toolbox hands its tool.

use scorsese_render::Cancel;
use scorsese_server::tools::Client;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, document, member, post, stored};

#[sqlx::test]
async fn a_queued_render_is_stopped_by_job_cancel_and_only_by_its_owner(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let (_, bob) = member(&pool, "bob@example.com").await;
    let id = stored(&pool, ana, json!({})).await;
    let (queued, _) = call(address, &token, "render", json!({ "project": id })).await;
    let job: i64 = queued
        .split_whitespace()
        .skip_while(|word| *word != "job")
        .nth(1)
        .and_then(|word| word.trim_end_matches(',').parse().ok())
        .unwrap_or_else(|| panic!("no job in {queued}"));

    let (said, refused) = call(address, &bob, "job_cancel", json!({ "job": job })).await;
    assert!(refused && said.contains("no such job"), "{said}");
    let (said, refused) = call(address, &token, "job_cancel", json!({ "job": job })).await;
    assert!(!refused && said.contains("cancelled"), "{said}");
    let (said, _) = call(address, &token, "jobs", json!({ "job": job })).await;
    assert!(said.contains("cancelled before it started"), "{said}");
}

#[sqlx::test]
async fn a_call_cancelled_before_it_runs_changes_nothing(pool: PgPool) {
    let (address, state) = common::serve_with(pool.clone(), common::files("mcp-cancel")).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, json!({})).await;
    let cancel = Cancel::new();
    cancel.cancel();
    let new_track = json!({ "project": id, "kind": "video" });
    let refused = state
        .tools
        .call_cancellable(ana, Client::External, "track_new", &new_track, &cancel)
        .await;
    assert!(refused.is_err_and(|why| why.contains("cancelled")));
    assert_eq!(document(&pool, ana, id).await.summary.revision, 1);

    // A cancel for a call nobody is serving is a notification like any other.
    let note = json!({ "jsonrpc": "2.0", "method": "notifications/cancelled",
                       "params": { "requestId": 7, "reason": "changed my mind" } });
    assert_eq!(post(address, &token, &note).await.status, 202);
}
