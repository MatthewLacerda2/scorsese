//! A message the person sent while the work was on its way takes the place
//! of the wake: the note waits for their next message. Work that landed while
//! nothing listened is found when the waker starts.

use sqlx::postgres::PgPool;

use super::{land, nth_turn, ordered, settle, turns, waking};
use crate::{Script, answers, exchange};

#[sqlx::test]
async fn a_message_sent_meanwhile_carries_the_note_instead(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, ana, who, state, queued) = ordered(&pool, &script).await;
    waking(&state);
    let project: i64 = sqlx::query_scalar("SELECT max(id) FROM projects")
        .fetch_one(&pool)
        .await
        .unwrap();
    script.replace(vec![answers("Still on their way.")]);
    exchange(address, &who, project, "how is it going?").await;
    for job in &queued {
        land(&pool, &state, ana, *job, "done").await;
    }
    settle().await;
    assert_eq!(turns(&pool).await, 2, "they spoke, so nothing wakes");

    script.replace(vec![answers("Both are in.")]);
    let next = exchange(address, &who, project, "and now?").await;
    assert_eq!(next["turn"]["woken"], false);
    let told = script
        .messages(script.requests().len() - 1)
        .last()
        .unwrap()
        .clone();
    assert!(told.contains("has finished"), "{told}");
    assert!(told.contains("line `outro`: made"), "{told}");
    assert!(!told.contains("The person has not written"), "{told}");
}

#[sqlx::test]
async fn work_that_landed_unheard_is_found_when_the_waker_starts(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, ana, who, state, queued) = ordered(&pool, &script).await;
    for job in &queued {
        land(&pool, &state, ana, *job, "done").await;
    }
    settle().await;
    assert_eq!(turns(&pool).await, 1, "nobody was listening");

    script.replace(vec![answers("Both lines are in.")]);
    waking(&state);
    let woken = nth_turn(&pool, address, &who, 2).await;
    assert_eq!(woken["turn"]["woken"], true, "{woken}");
}

#[sqlx::test]
async fn a_stopped_conversation_is_not_woken(pool: PgPool) {
    let script = Script::new(vec![]);
    let (_, ana, _, state, queued) = ordered(&pool, &script).await;
    sqlx::query("UPDATE chat_turns SET state = 'stopped'")
        .execute(&pool)
        .await
        .unwrap();
    waking(&state);
    for job in &queued {
        land(&pool, &state, ana, *job, "done").await;
    }
    settle().await;
    assert_eq!(turns(&pool).await, 1, "the person stopped it");
}

#[sqlx::test]
async fn a_conversation_a_new_one_set_aside_is_not_woken(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, ana, who, state, queued) = ordered(&pool, &script).await;
    waking(&state);
    let project: i64 = sqlx::query_scalar("SELECT max(id) FROM projects")
        .fetch_one(&pool)
        .await
        .unwrap();
    script.replace(vec![answers("Starting over.")]);
    let path = format!("/api/projects/{project}/chat");
    let body = serde_json::json!({ "prompt": "start again", "fresh": true });
    let sent = crate::common::request(address, "POST", &path, &[&who], Some(&body)).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    nth_turn(&pool, address, &who, 2).await;
    for job in &queued {
        land(&pool, &state, ana, *job, "done").await;
    }
    settle().await;
    assert_eq!(turns(&pool).await, 2, "the old conversation stays asleep");
    let awaiting: i64 = sqlx::query_scalar("SELECT count(*) FROM chat_turns WHERE awaiting")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(awaiting, 0, "and stops waiting");
}

#[sqlx::test]
async fn work_landing_while_the_turn_runs_wakes_once_it_ends(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, ana, who, state, queued) = ordered(&pool, &script).await;
    sqlx::query("UPDATE chat_turns SET state = 'running'")
        .execute(&pool)
        .await
        .unwrap();
    waking(&state);
    for job in &queued {
        land(&pool, &state, ana, *job, "done").await;
    }
    settle().await;
    assert_eq!(turns(&pool).await, 1, "the turn is still running");

    script.replace(vec![answers("Both lines are in.")]);
    let id: i64 = sqlx::query_scalar("UPDATE chat_turns SET state = 'answered' RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    let ended = scorsese_server::assistant::detail(&pool, ana, id).await;
    let turn = ended.expect("the test setup holds").turn;
    let event = scorsese_server::events::Event::ChatTurn {
        turn,
        balance_micros: 0,
    };
    state.events.send(ana, event);
    let woken = nth_turn(&pool, address, &who, 2).await;
    assert_eq!(woken["turn"]["woken"], true, "{woken}");
}
