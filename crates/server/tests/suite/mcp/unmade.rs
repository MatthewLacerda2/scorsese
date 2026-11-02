//! A vendor call that completes with no output (#1646): the model answered in
//! words instead of a picture or a line. The vendor bills it, so it is
//! charged as quoted, and the member is told what the model said — while a
//! call the vendor did not complete is still free.

use scorsese_core::GenerationState;
use scorsese_server::credits::{dollars, ledger};
use scorsese_server::db::{self, UserId};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::paying::{fund, narrated, quote_and_confirm, token};
use super::vendors::{Mock, serve, settled};
use super::{call, document, member};

/// What a 2K 16:9 still is charged: $0.06, times 1.8.
const STILL: i64 = 108_000;

/// A project with one sketched still in it.
fn sketched() -> Value {
    json!({ "assets": [
        { "id": "backdrop", "kind": "generated_image", "state": "sketch",
          "prompt": "A harbour at dusk, wide." }
    ] })
}

/// `user`'s balance, in micro-dollars.
async fn balance(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    let balance = ledger::balance(&mut tx).await.expect("a balance");
    tx.commit().await.expect("the read commits");
    balance
}

/// What the one generation job ended saying.
async fn said(pool: &PgPool) -> String {
    let error: Option<String> =
        sqlx::query_scalar("SELECT error FROM jobs WHERE kind <> 'thumbnail'")
            .fetch_one(pool)
            .await
            .expect("the job");
    error.unwrap_or_default()
}

/// `mock` drawing `sketched`'s still for a funded member, now or in a batch:
/// the member, and what their balance fell by.
async fn drawn(pool: &PgPool, mock: &Mock, batch: bool) -> (UserId, i64) {
    let (address, state, _worker) = serve(pool, mock).await;
    let (ana, who) = member(pool, "ana@example.com").await;
    let id = super::stored(pool, ana, sketched()).await;
    fund(pool, ana, 10).await;
    let ask = json!({ "project": id, "batch": batch });
    let (quote, refused) = call(address, &who, "generate", ask).await;
    assert!(!refused, "{quote}");
    let yes = json!({ "project": id, "batch": batch, "confirm": token(&quote) });
    let (queued, refused) = call(address, &who, "generate", yes).await;
    assert!(!refused, "{queued}");
    assert_eq!(settled(&state, ana).await, vec!["failed"]);
    let stored = document(pool, ana, id).await.document;
    assert_eq!(
        stored.assets[0].state,
        Some(GenerationState::Sketch),
        "nothing was drawn, so nothing lands"
    );
    (ana, 10_000_000 - balance(pool, ana).await)
}

#[sqlx::test]
async fn a_still_answered_in_words_is_charged_as_quoted(pool: PgPool) {
    let mock = Mock {
        say: Some("I can't draw that.".into()),
        ..Mock::default()
    };
    let (_, spent) = drawn(&pool, &mock, false).await;
    assert_eq!(spent, STILL, "charged as quoted, not released");
    let said = said(&pool).await;
    assert!(
        said.contains("without a picture")
            && said.contains("I can't draw that.")
            && said.contains(&format!("charged {}", dollars(STILL))),
        "{said}"
    );
    let (state_of, charged): (String, i64) = sqlx::query_as(
        "SELECT g.state, (SELECT count(*) FROM credit_entries WHERE kind = 'charge')
         FROM image_generations g",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((state_of.as_str(), charged), ("failed", 1));
}

#[sqlx::test]
async fn a_still_the_vendor_failed_is_still_free(pool: PgPool) {
    let mock = Mock {
        fail: Some("the service is unavailable".into()),
        ..Mock::default()
    };
    let (_, spent) = drawn(&pool, &mock, false).await;
    assert_eq!(spent, 0, "a call the vendor did not complete is released");
    let said = said(&pool).await;
    assert!(said.contains("nothing was charged"), "{said}");
}

#[sqlx::test]
async fn a_batched_still_answered_in_words_is_charged_its_half(pool: PgPool) {
    let mock = Mock {
        say: Some("I can't draw that.".into()),
        ..Mock::default()
    };
    let (_, spent) = drawn(&pool, &mock, true).await;
    assert_eq!(spent, STILL / 2, "charged its half price, not released");
    let said = said(&pool).await;
    assert!(said.contains("I can't draw that."), "{said}");
}

#[sqlx::test]
async fn a_batched_still_the_vendor_failed_is_still_free(pool: PgPool) {
    let mock = Mock {
        fail: Some("the request was invalid".into()),
        ..Mock::default()
    };
    let (_, spent) = drawn(&pool, &mock, true).await;
    assert_eq!(spent, 0);
}

#[sqlx::test]
async fn a_line_answered_without_audio_is_charged_as_quoted(pool: PgPool) {
    let mock = Mock {
        say: Some("the audio came back as something not base64".into()),
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, narrated()).await;
    fund(&pool, ana, 10).await;
    let (queued, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{queued}");
    assert_eq!(settled(&state, ana).await, vec!["failed"]);
    let spent = 10_000_000 - balance(&pool, ana).await;
    assert!(spent > 0, "the line was charged");
    let said = said(&pool).await;
    assert!(
        said.contains("without a line")
            && said.contains("not base64")
            && said.contains(&format!("charged {}", dollars(spent))),
        "{said}"
    );
}
