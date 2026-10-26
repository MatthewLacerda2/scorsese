//! `voice_design` on the web (#572): quoted, paid from credits, made by a job,
//! its samples kept in the library and never paid for twice — against a
//! studio that never spends a cent.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::{fund, token};
use super::vendors::{Mock, serve, settled};
use super::{call, member};

const PROMPT: &str = "A warm, unhurried radio voice in her fifties, for late-night stories.";
const TEXT: &str = "Every city has a night editor, and every night editor has a story about \
                    the one that got away. This is that story, told the way she tells it.";

#[sqlx::test]
async fn a_design_is_quoted_paid_kept_and_never_paid_for_twice(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let brief = json!({ "prompt": PROMPT, "text": TEXT, "seed": 7 });

    let (quote, refused) = call(address, &who, "voice_design", brief.clone()).await;
    assert!(!refused && quote.contains("charged once"), "{quote}");
    let poor = json!({ "confirm": token(&quote) });
    let (said, refused) = call(address, &who, "voice_design", poor).await;
    assert!(refused && said.contains("Add credit"), "{said}");
    assert_eq!(mock.designed(), 0, "a refused spend sends nothing");

    // Funded, and confirmed with the token alone — as the assistant's
    // confirmation box confirms.
    fund(&pool, ana, 10).await;
    let (quote, _) = call(address, &who, "voice_design", brief.clone()).await;
    let yes = json!({ "confirm": token(&quote) });
    let (said, refused) = call(address, &who, "voice_design", yes).await;
    assert!(!refused && said.starts_with("Designing as job "), "{said}");
    assert_eq!(settled(&state, ana).await, ["done"]);
    assert_eq!(mock.designed(), 1);

    let charged: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM credit_entries WHERE kind = 'charge' AND voice_design_id IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("the charges count");
    assert_eq!(charged, 1, "the design was charged, once");

    // The same brief again: answered from the library, free, with no token.
    let (again, refused) = call(address, &who, "voice_design", brief).await;
    assert!(!refused && again.contains("Already designed"), "{again}");
    assert!(
        again.contains("1. gen-1") && again.contains("library item"),
        "{again}"
    );
    assert!(!again.contains("confirm"), "{again}");
    assert_eq!(
        mock.designed(),
        1,
        "an unchanged design is never paid twice"
    );

    let keep = json!({ "keep": "gen-2", "name": "Night editor" });
    let (said, refused) = call(address, &who, "voice_design", keep).await;
    assert!(!refused && said.starts_with("Keeping gen-2"), "{said}");
    assert_eq!(settled(&state, ana).await, ["done", "done"]);
    assert_eq!(mock.kept(), 1);
    let (listed, _) = call(address, &who, "voice_design", json!({ "list": true })).await;
    assert!(
        listed.contains("voice-of-gen-2  Night editor  (seed 7)"),
        "{listed}"
    );
    assert!(listed.contains(PROMPT), "{listed}");
}

#[sqlx::test]
async fn a_design_is_its_owners_alone(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (bob, his) = member(&pool, "bob@example.com").await;
    fund(&pool, ana, 10).await;
    fund(&pool, bob, 10).await;
    let brief = json!({ "prompt": PROMPT, "text": TEXT });
    let (quote, _) = call(address, &hers, "voice_design", brief.clone()).await;
    let (said, refused) = call(
        address,
        &his,
        "voice_design",
        json!({ "confirm": token(&quote) }),
    )
    .await;
    assert!(
        refused && said.contains("not an unspent voice_design quote"),
        "{said}"
    );
    call(
        address,
        &hers,
        "voice_design",
        json!({ "confirm": token(&quote) }),
    )
    .await;
    settled(&state, ana).await;

    let keep = json!({ "keep": "gen-1", "name": "Mine now" });
    let (said, refused) = call(address, &his, "voice_design", keep).await;
    assert!(refused && said.contains("none of your designs"), "{said}");
    // Bob's own design of the same brief is his to pay for: Ana's samples are
    // not his.
    let (quote, _) = call(address, &his, "voice_design", brief).await;
    assert!(quote.contains("confirm: \""), "{quote}");
    assert!(settled(&state, bob).await.is_empty());
    assert_eq!(mock.designed(), 1);
}
