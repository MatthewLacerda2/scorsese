//! A job's owner hears every change to it as it happens, and nobody else does.

use std::time::Duration;

use futures_util::StreamExt;
use scorsese_server::accounts::tokens;
use scorsese_server::events::{Event, Events};
use scorsese_server::http::{self, AppState};
use scorsese_server::jobs::{Outcome, Queue, Registry, State};
use serde_json::json;
use sqlx::postgres::PgPool;
use tokio::time::timeout;

use super::{ECHO, account, common, enqueue, members, start_worker, stop_worker};
use common::stream::{listen, read_until};

#[sqlx::test]
async fn the_owner_hears_a_job_wait_run_and_finish(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let events = Events::new();
    let queue = Queue::new(events.clone());
    let mut heard = Box::pin(events.subscribe(ana));
    let registry = Registry::new().register(
        ECHO,
        |job: scorsese_server::jobs::Job, _context| async move { Outcome::Done(job.payload) },
    );
    let worker = start_worker(&members, registry, &queue);

    let job = enqueue(&members, ana, ECHO, json!({ "n": 7 })).await;
    queue.announce(ana, &job);
    let mut states = Vec::new();
    while states.last() != Some(&State::Done) {
        let next = timeout(Duration::from_secs(10), heard.next())
            .await
            .unwrap();
        let Some(Event::Job(view)) = next else {
            panic!("{next:?}")
        };
        assert_eq!(view.id, job.id);
        states.push(view.state);
    }
    stop_worker(worker).await;
    assert_eq!(states, [State::Waiting, State::Running, State::Done]);
}

#[sqlx::test]
async fn jobs_and_their_stream_are_the_callers_alone(pool: PgPool) {
    let (ana, bia) = (
        account(&pool, "ana@example.com").await,
        account(&pool, "bia@example.com").await,
    );
    let members = members(&pool).await;
    let ana_token = tokens::issue(&pool, ana, "t").await.unwrap().token;
    let bia_token = tokens::issue(&pool, bia, "t").await.unwrap().token;
    let state = AppState::new(members.clone(), common::files("live"));
    let (listener, address) = common::listener().await;
    let router = http::router(state.clone());
    let server = tokio::spawn(http::serve(listener, router, std::future::pending()));
    common::release_at_teardown(&pool, &members, [server.abort_handle()]);

    let anonymous = common::request(address, "GET", "/api/events", &[], None).await;
    assert_eq!(anonymous.status, 401);
    let mut ana_hears = listen(address, &format!("Authorization: Bearer {ana_token}")).await;
    let mut bia_hears = listen(address, &format!("Authorization: Bearer {bia_token}")).await;

    let job = enqueue(&members, ana, ECHO, json!({})).await;
    state.jobs.announce(ana, &job);
    let said = read_until(&mut ana_hears, "\n\n", Duration::from_secs(5)).await;
    let said = said.expect("ana hears about her job");
    assert!(said.contains(r#""type":"job""#), "{said}");
    assert!(said.contains(&format!(r#""id":{}"#, job.id)), "{said}");
    let overheard = read_until(&mut bia_hears, "data:", Duration::from_millis(300)).await;
    assert_eq!(overheard, None);

    let ana_auth = format!("Authorization: Bearer {ana_token}");
    let bia_auth = format!("Authorization: Bearer {bia_token}");
    let path = format!("/api/jobs/{}", job.id);
    let listed = common::request(address, "GET", "/api/jobs", &[&ana_auth], None).await;
    assert_eq!(listed.json()[0]["id"], job.id);
    let theirs = common::request(address, "GET", &path, &[&bia_auth], None).await;
    assert_eq!(theirs.status, 404);
    let empty = common::request(address, "GET", "/api/jobs", &[&bia_auth], None).await;
    assert_eq!(empty.json(), json!([]));
}
