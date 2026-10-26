//! A woken turn past the threshold compacts like any other, and reopens from
//! the summary with the server's words, never the transcript's marker as the
//! person's (#1489).

use sqlx::postgres::PgPool;

use super::super::compaction::summarising;
use super::{Script, answers, land, nth_turn, ordered, waking};

#[sqlx::test]
async fn a_woken_turn_that_compacts_reopens_with_the_servers_words(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, ana, who, state, queued) = ordered(&pool, &script).await;
    // As if the conversation had grown past the threshold by its last call.
    sqlx::query("UPDATE model_calls SET cache_read_tokens = 150001 WHERE id = (SELECT max(id) FROM model_calls)")
        .execute(&pool)
        .await
        .unwrap();
    script.replace(vec![
        answers("SUMMARY: two lines ordered for the intro."),
        answers("Both lines are narrated."),
    ]);
    waking(&state);
    land(&pool, &state, ana, queued[0], "done").await;
    land(&pool, &state, ana, queued[1], "done").await;
    let woken = nth_turn(&pool, address, &who, 2).await;
    assert_eq!(woken["turn"]["woken"], true, "{woken}");
    assert_eq!(woken["turn"]["prompt"], "Continued after the lines landed");
    assert_eq!(
        woken["turn"]["answer"], "Both lines are narrated.",
        "{woken}"
    );

    let last = script.requests().len() - 1;
    assert!(summarising(&script, last - 1), "it compacted first");
    let opened = script.parsed(last);
    let words = opened[0]["content"][0]["text"].as_str().unwrap();
    assert!(words.contains("The person has not written"), "{opened:?}");
    let sent = script.messages(last);
    assert!(sent.concat().contains("SUMMARY: two lines"), "{sent:?}");
    assert!(
        sent.iter()
            .all(|message| !message.contains("Continued after")),
        "the marker is never sent as the person's words: {sent:?}"
    );
}
