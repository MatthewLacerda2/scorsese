//! A continuation refused while the person ran as many turns as they may is
//! started once one of theirs ends, in whichever project it ran.

use scorsese_server::events::Event;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::{Script, answers, exchange, project};
use super::{land, nth_turn, ordered, settle, turns, waking};

#[sqlx::test]
async fn a_continuation_refused_for_room_starts_once_another_turn_ends(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, ana, who, state, queued) = ordered(&pool, &script).await;
    // Two turns of ana's elsewhere, held running.
    let mut elsewhere = Vec::new();
    for _ in 0..2 {
        let id = project(&pool, ana, json!([])).await;
        script.replace(vec![answers("Done.")]);
        let detail = exchange(address, &who, id, "and this").await;
        elsewhere.push(detail["turn"]["id"].as_i64().unwrap());
    }
    sqlx::query("UPDATE chat_turns SET state = 'running' WHERE id = ANY($1)")
        .bind(&elsewhere)
        .execute(&pool)
        .await
        .unwrap();
    waking(&state);
    for job in &queued {
        land(&pool, &state, ana, *job, "done").await;
    }
    settle().await;
    assert_eq!(turns(&pool).await, 3, "no room for the continuation");

    script.replace(vec![answers("Both lines are in.")]);
    sqlx::query("UPDATE chat_turns SET state = 'answered' WHERE id = $1")
        .bind(elsewhere[0])
        .execute(&pool)
        .await
        .unwrap();
    let ended = scorsese_server::assistant::detail(&pool, ana, elsewhere[0])
        .await
        .unwrap()
        .turn;
    state.events.send(
        ana,
        Event::ChatTurn {
            turn: ended,
            balance_micros: 0,
        },
    );
    let woken = nth_turn(&pool, address, &who, 4).await;
    assert_eq!(woken["turn"]["woken"], true, "{woken}");
    assert_eq!(woken["turn"]["prompt"], "Continued after the lines landed");
}
