//! What the woken assistant is told of each job: the asset its result names,
//! which a migration may have renamed since the job was queued, and why a
//! stuck one is stuck — its reservation still held.

use scorsese_server::db::UserId;
use scorsese_server::events::Event;
use scorsese_server::http::AppState;
use scorsese_server::jobs::store as jobs;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::super::{Script, answers};
use super::{nth_turn, ordered, waking};

#[sqlx::test]
async fn the_note_names_the_landed_asset_and_why_a_job_is_stuck(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, ana, who, state, queued) = ordered(&pool, &script).await;
    waking(&state);
    let result = json!({ "asset": "vo-2" });
    ended(&pool, &state, ana, queued[0], "done", Some(result), None).await;
    script.replace(vec![answers("The intro is narrated; the outro is held.")]);
    let why = "no ElevenLabs key is set";
    ended(&pool, &state, ana, queued[1], "stuck", None, Some(why)).await;

    nth_turn(&pool, address, &who, 2).await;
    let sent = script.messages(script.requests().len() - 1);
    let told = sent.last().expect("the note").clone();
    assert!(told.contains("line `vo-2`: made"), "{told}");
    assert!(told.contains("line `outro`: stuck"), "{told}");
    assert!(told.contains("held") && told.contains(why), "{told}");
    assert!(!told.contains("took too long"), "{told}");
}

/// End job `job` in `state_of`, with `result` or `error`, as the worker
/// records it and tells its owner.
async fn ended(
    pool: &PgPool,
    state: &AppState,
    user: UserId,
    job: i64,
    state_of: &str,
    result: Option<Value>,
    error: Option<&str>,
) {
    sqlx::query(
        "UPDATE jobs SET state = $2, finished_at = now(), result = $3, error = $4
         WHERE id = $1",
    )
    .bind(job)
    .bind(state_of)
    .bind(result)
    .bind(error)
    .execute(pool)
    .await
    .expect("the test setup holds");
    let view = jobs::get(&state.pool, user, job)
        .await
        .expect("the test setup holds")
        .expect("the test setup holds");
    state.events.send(user, Event::Job(view));
}
