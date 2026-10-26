//! When the work a conversation ordered lands, the assistant is woken and
//! carries on by itself (#1441) — once the last of it has ended, and not when
//! the person wrote meanwhile.

mod compacted;
mod meanwhile;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use scorsese_server::assistant;
use scorsese_server::db::UserId;
use scorsese_server::events::Event;
use scorsese_server::http::AppState;
use scorsese_server::jobs::store as jobs;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::{Script, answers, calls, exchange, finished, member, project, scripted};

/// Two lines ready to speak.
fn narrated() -> Value {
    json!([
        { "id": "vo", "kind": "generated_audio", "state": "sketch",
          "prompt": "Every city has a night editor.", "speech": { "voice_id": "voice-one" } },
        { "id": "outro", "kind": "generated_audio", "state": "sketch",
          "prompt": "Goodnight.", "speech": { "voice_id": "voice-one" } }
    ])
}

/// A conversation whose turn quoted both lines and heard a yes, ending with
/// its two jobs queued: the address, the user, their `Authorization:` line,
/// the server's state and the jobs, oldest first.
async fn ordered(
    pool: &PgPool,
    script: &Arc<Script>,
) -> (SocketAddr, UserId, String, AppState, Vec<i64>) {
    let (address, state) = scripted(pool, script).await;
    let (ana, who) = member(pool, "ana@example.com", 10).await;
    let id = project(pool, ana, narrated()).await;
    script.replace(vec![calls("generate", json!({ "project": id }))]);
    let detail = exchange(address, &who, id, "narrate it").await;
    script.replace(vec![answers("Both lines are on their way.")]);
    let path = format!("/api/chat/turns/{}/quote", detail["turn"]["id"]);
    let body = json!({ "confirm": true });
    let yes = super::common::request(address, "POST", &path, &[&who], Some(&body)).await;
    assert_eq!(yes.status, 200, "{}", yes.body);
    finished(
        address,
        &who,
        detail["turn"]["id"].as_i64().expect("the test setup holds"),
    )
    .await;
    let queued: Vec<i64> = sqlx::query_scalar("SELECT id FROM jobs ORDER BY id")
        .fetch_all(pool)
        .await
        .expect("the test setup holds");
    assert_eq!(queued.len(), 2, "one job a line");
    (address, ana, who, state, queued)
}

/// Run the waker beside the server for the rest of the test.
fn waking(state: &AppState) {
    let (stop, stopping) = watch::channel(false);
    tokio::spawn(assistant::watch(state.clone(), stopping));
    // Kept for the test's life: dropped, the waker would read it as a stop.
    std::mem::forget(stop);
}

/// End job `job` in `outcome` (`done` or `failed`), as the worker records it
/// and tells its owner.
async fn land(pool: &PgPool, state: &AppState, user: UserId, job: i64, outcome: &str) {
    sqlx::query(
        "UPDATE jobs SET state = $2, finished_at = now(),
                error = CASE WHEN $2 = 'failed' THEN 'the voice is gone' END
         WHERE id = $1",
    )
    .bind(job)
    .bind(outcome)
    .execute(pool)
    .await
    .expect("the test setup holds");
    let view = jobs::get(&state.pool, user, job)
        .await
        .expect("the test setup holds")
        .expect("the test setup holds");
    state.events.send(user, Event::Job(view));
}

/// How many turns there are.
async fn turns(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM chat_turns")
        .fetch_one(pool)
        .await
        .expect("the test setup holds")
}

/// The newest turn once there are `n`, finished: its detail.
async fn nth_turn(pool: &PgPool, address: SocketAddr, who: &str, n: i64) -> Value {
    for _ in 0..200 {
        if turns(pool).await >= n {
            let id: i64 = sqlx::query_scalar("SELECT max(id) FROM chat_turns")
                .fetch_one(pool)
                .await
                .expect("the test setup holds");
            return finished(address, who, id).await;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("there are never {n} turns");
}

/// Long enough for a waker that was going to start a turn to have started it.
async fn settle() {
    tokio::time::sleep(Duration::from_millis(300)).await;
}

#[sqlx::test]
async fn the_last_job_landing_wakes_the_conversation_with_what_landed(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, ana, who, state, queued) = ordered(&pool, &script).await;
    waking(&state);
    land(&pool, &state, ana, queued[0], "done").await;
    settle().await;
    assert_eq!(turns(&pool).await, 1, "one job is still on its way");

    script.replace(vec![answers("The intro is narrated; the outro failed.")]);
    // Changed after the order: the woken turn runs at the project's level.
    sqlx::query("UPDATE projects SET assistant_effort = 'medium'")
        .execute(&pool)
        .await
        .unwrap();
    land(&pool, &state, ana, queued[1], "failed").await;
    let woken = nth_turn(&pool, address, &who, 2).await;
    let turn = &woken["turn"];
    assert_eq!(turn["woken"], true, "{woken}");
    assert_eq!(turn["prompt"], "Continued after the lines landed");
    assert_eq!(turn["state"], "answered");
    let sent = script.messages(script.requests().len() - 1);
    let told = sent.last().unwrap().clone();
    assert!(told.contains("The person has not written"), "{told}");
    assert!(
        sent.iter()
            .all(|message| !message.contains("Continued after")),
        "the transcript's line is never sent as the person's words: {sent:?}"
    );
    assert!(told.contains("line `vo`: made"), "{told}");
    assert!(told.contains("line `outro`: failed"), "{told}");
    assert!(told.contains("the voice is gone"), "{told}");
    assert!(
        !told.contains("job "),
        "job numbers mean nothing to it: {told}"
    );
    let effort: Vec<String> = sqlx::query_scalar("SELECT effort FROM chat_turns ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        effort,
        ["high", "medium"],
        "the project's level, not the order's"
    );

    settle().await;
    assert_eq!(turns(&pool).await, 2, "woken once");
}
