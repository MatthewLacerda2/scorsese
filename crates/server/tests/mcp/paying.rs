//! `generate` on the web: quote first, spend only on a token, from the user's
//! own credits — all of it, or none of it.

use scorsese_server::credits::ledger;
use scorsese_server::db::{self, UserId};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{call, common, member};

/// A project with one line ready to speak and one with no voice yet.
pub(super) fn narrated() -> Value {
    json!({ "assets": [
        { "id": "vo", "kind": "generated_audio", "state": "sketch",
          "prompt": "Every city has a night editor.", "speech": { "voice_id": "voice-one" } },
        { "id": "vo2", "kind": "generated_audio", "state": "sketch", "prompt": "Later." }
    ] })
}

/// `dollars` of credit for `user`.
pub(super) async fn fund(pool: &PgPool, user: UserId, dollars: i64) {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    ledger::top_up(&mut tx, dollars * 1_000_000)
        .await
        .expect("a top-up is recorded");
    tx.commit().await.expect("the top-up commits");
}

/// The token a quote names.
pub(super) fn token(quote: &str) -> String {
    quote
        .split("confirm: \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_else(|| panic!("no token in {quote}"))
        .to_owned()
}

/// Quote `project`, then confirm the quote: what the confirmation said.
pub(super) async fn quote_and_confirm(
    address: std::net::SocketAddr,
    who: &str,
    project: i64,
) -> (String, bool) {
    let (quote, _) = call(address, who, "generate", json!({ "project": project })).await;
    let confirm = json!({ "project": project, "confirm": token(&quote) });
    call(address, who, "generate", confirm).await
}

#[sqlx::test]
async fn a_quote_spends_nothing_and_a_yes_reserves_and_queues(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, narrated()).await;

    let (quote, refused) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(!refused, "{quote}");
    assert!(quote.contains("vo: $0.0"), "{quote}");
    assert!(quote.contains("vo2: not yet"), "{quote}");
    assert!(quote.contains("Your balance is $0.00"), "{quote}");

    // No money: refused, nothing queued — and the token is spent all the same.
    let spend = json!({ "project": id, "confirm": token(&quote) });
    let (said, refused) = call(address, &who, "generate", spend.clone()).await;
    assert!(refused && said.contains("Add credit"), "{said}");
    let (said, refused) = call(address, &who, "generate", spend).await;
    assert!(refused && said.contains("not a live quote"), "{said}");
    assert_eq!(jobs(&pool, ana).await, 0);

    fund(&pool, ana, 10).await;
    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused && said.starts_with("vo: queued as job "), "{said}");
    assert_eq!(jobs(&pool, ana).await, 1);
    let (call_id, job): (Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT tool_call_id, job_id FROM speech_generations")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        call_id.is_some() && job.is_some(),
        "the audit row names both"
    );

    // Being made already: a second yes cannot pay for it again.
    let (again, _) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(again.contains("already on its way as job"), "{again}");
    assert!(again.contains("Nothing to pay for"), "{again}");
}

#[sqlx::test]
async fn a_token_is_its_owners_alone(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (bob, his) = member(&pool, "bob@example.com").await;
    let (a, b) = (
        super::stored(&pool, ana, narrated()).await,
        super::stored(&pool, bob, narrated()).await,
    );
    fund(&pool, bob, 10).await;
    let (quote, _) = call(address, &hers, "generate", json!({ "project": a })).await;
    let stolen = json!({ "project": b, "confirm": token(&quote) });
    let (said, refused) = call(address, &his, "generate", stolen).await;
    assert!(refused && said.contains("not a live quote"), "{said}");
    assert_eq!(jobs(&pool, bob).await, 0);
}

/// How many generation jobs `user` has.
async fn jobs(pool: &PgPool, user: UserId) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM jobs WHERE user_id = $1 AND kind = 'spoken_line'")
        .bind(user.get())
        .fetch_one(pool)
        .await
        .expect("the jobs count")
}
