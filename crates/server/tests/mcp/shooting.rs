//! A Veo shot as a job: sent once and kept when it works, free when refused
//! or lost.

use scorsese_core::GenerationState;
use scorsese_server::credits::ledger;
use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::{fund, quote_and_confirm};
use super::vendors::{Mock, serve, settled};
use super::{document, member};

#[sqlx::test]
async fn a_refused_shot_costs_nothing(pool: PgPool) {
    let mock = Mock {
        refuse: Some("the prompt was declined".into()),
        ..Mock::default()
    };
    ends_free(pool, mock, "declined").await;
}

/// A ticket Veo will never answer for settles at once, as a refusal does,
/// rather than being polled until patience runs out (#1196).
#[sqlx::test]
async fn a_lost_shot_settles_like_a_refusal(pool: PgPool) {
    let mock = Mock {
        lose: Some("the operation was not found".into()),
        ..Mock::default()
    };
    ends_free(pool, mock, "no longer knows").await;
}

/// A shot confirmed against `mock`, which fails it saying `why`: the job ends
/// failed, the ticket is kept, and nothing is charged.
async fn ends_free(pool: PgPool, mock: Mock, why: &str) {
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let shot = json!({ "assets": [
        { "id": "hero", "kind": "generated_video", "state": "sketch",
          "prompt": "a lighthouse in a storm" }
    ] });
    let id = super::stored(&pool, ana, shot).await;
    fund(&pool, ana, 10).await;

    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(
        !refused && said.starts_with("hero: queued as job "),
        "{said}"
    );
    assert_eq!(settled(&state, ana).await, vec!["failed"]);
    assert_eq!(mock.submitted(), 1);

    let error: Option<String> =
        sqlx::query_scalar("SELECT error FROM jobs WHERE kind = 'veo_shot'")
            .fetch_one(&pool)
            .await
            .expect("the job");
    let error = error.unwrap_or_default();
    assert!(
        error.contains(why) && error.contains("nothing was charged"),
        "{error}"
    );
    let ticket: Option<String> = sqlx::query_scalar("SELECT ticket FROM veo_generations")
        .fetch_one(&pool)
        .await
        .expect("the generation");
    assert_eq!(
        ticket.as_deref(),
        Some("operations/mock-1"),
        "the ticket was kept"
    );

    let mut tx = db::scoped(&pool, ana).await.expect("scoped");
    assert_eq!(
        ledger::balance(&mut tx).await.expect("a balance"),
        10_000_000,
        "free"
    );
    tx.commit().await.expect("committed");
    let hero = &document(&pool, ana, id).await.document.assets[0];
    assert_eq!(hero.state, Some(GenerationState::Sketch));
}

#[sqlx::test]
async fn a_shot_is_sent_once_and_lands(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let shot = json!({ "assets": [
        { "id": "hero", "kind": "generated_video", "state": "sketch",
          "prompt": "a lighthouse at dawn" }
    ] });
    let id = super::stored(&pool, ana, shot).await;
    fund(&pool, ana, 10).await;
    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);
    assert_eq!(mock.submitted(), 1);
    let hero = &document(&pool, ana, id).await.document.assets[0];
    assert_eq!(hero.state, Some(GenerationState::Generated));
    assert!(hero.operation.is_none());
    let path = hero.path.as_ref().expect("a path").as_str().to_owned();
    assert!(
        path.starts_with("generated/hero-") && path.ends_with(".mp4"),
        "{path}"
    );
    assert!(
        hero.media.is_some_and(|media| media.width == Some(64)),
        "measured"
    );
    let state_of: String = sqlx::query_scalar("SELECT state FROM veo_generations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state_of, "generated");
}
