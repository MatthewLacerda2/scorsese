//! A generation in flight across a schema bump: the job reads the document it
//! carries through the migration, lands on the asset the migration renamed,
//! and is charged once — and one Veo had already taken is never settled free.

mod queued;

use scorsese_core::{GenerationState, Project};
use scorsese_server::credits::ledger;
use scorsese_server::db::{self, UserId};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::document;
use super::vendors::{Mock, settled};
use queued::{Before, Carried, TICKET, enqueued_before_the_bump};

/// The id the stored project has, after the server migrated it.
const RENAMED: &str = "hero-shot";

/// The id a v55 build allowed, which the v55 → v56 step renames to
/// [`RENAMED`].
const WRITTEN: &str = "hero shot";

#[sqlx::test]
async fn a_shot_queued_before_the_bump_is_sent_and_lands_on_the_renamed_asset(pool: PgPool) {
    let mock = Mock::default();
    let before = enqueued_before_the_bump(&pool, &mock, Carried::At(55), None).await;
    assert_eq!(settled(&before.state, before.user).await, vec!["done"]);
    assert_eq!(mock.submitted(), 1);
    lands(&pool, &before).await;
}

#[sqlx::test]
async fn a_shot_veo_took_before_the_bump_is_polled_and_charged_once(pool: PgPool) {
    let mock = Mock::default();
    let before = enqueued_before_the_bump(&pool, &mock, Carried::At(55), Some(TICKET)).await;
    assert_eq!(settled(&before.state, before.user).await, vec!["done"]);
    assert_eq!(mock.submitted(), 0, "polled, never sent again");
    lands(&pool, &before).await;
}

/// A ticket is the record that Veo was paid: a job holding one whose
/// document this build cannot read keeps its reservation, for the operator.
#[sqlx::test]
async fn a_shot_veo_took_whose_document_does_not_migrate_keeps_its_reservation(pool: PgPool) {
    let mock = Mock::default();
    let before = enqueued_before_the_bump(&pool, &mock, Carried::At(10), Some(TICKET)).await;
    assert_eq!(settled(&before.state, before.user).await, vec!["stuck"]);
    held(&pool, &before, "does not load").await;
}

/// No key on the server is no refusal once Veo holds the shot.
#[sqlx::test]
async fn a_shot_veo_took_waits_out_a_missing_key(pool: PgPool) {
    let mock = Mock {
        keyless_video: true,
        ..Mock::default()
    };
    let before = enqueued_before_the_bump(&pool, &mock, Carried::Current, Some(TICKET)).await;
    assert_eq!(settled(&before.state, before.user).await, vec!["stuck"]);
    held(&pool, &before, "no Gemini key").await;
}

/// The shot was kept and charged what was reserved for it, once, and the
/// stored project's renamed asset points at it.
async fn lands(pool: &PgPool, before: &Before) {
    let state_of: String = sqlx::query_scalar("SELECT state FROM veo_generations")
        .fetch_one(pool)
        .await
        .expect("the generation");
    assert_eq!(state_of, "generated");
    assert_eq!(
        balance(pool, before.user).await,
        before.reserved,
        "charged once"
    );

    let stored: Project = document(pool, before.user, before.project).await.document;
    let hero = &stored.assets[0];
    assert_eq!(hero.id.as_str(), RENAMED);
    assert_eq!(hero.state, Some(GenerationState::Generated));
    let path = hero.path.as_ref().expect("a path").as_str().to_owned();
    assert!(path.starts_with(&format!("generated/{RENAMED}-")), "{path}");

    let result: Value = sqlx::query_scalar("SELECT result FROM jobs WHERE kind = 'veo_shot'")
        .fetch_one(pool)
        .await
        .expect("the job's result");
    assert_eq!(result["asset"], json!(RENAMED), "{result}");
    assert_eq!(result["adopted"], json!(true), "{result}");
}

/// The shot is still pending, its reservation held, and the job says why.
async fn held(pool: &PgPool, before: &Before, why: &str) {
    let state_of: String = sqlx::query_scalar("SELECT state FROM veo_generations")
        .fetch_one(pool)
        .await
        .expect("the generation");
    assert_eq!(state_of, "pending");
    assert_eq!(balance(pool, before.user).await, before.reserved, "held");
    let error: Option<String> =
        sqlx::query_scalar("SELECT error FROM jobs WHERE kind = 'veo_shot'")
            .fetch_one(pool)
            .await
            .expect("the job");
    let error = error.unwrap_or_default();
    assert!(
        error.contains(why) && !error.contains("nothing was charged"),
        "{error}"
    );
}

/// `user`'s paid balance.
async fn balance(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.expect("scoped");
    let balance = ledger::balance(&mut tx).await.expect("a balance");
    tx.commit().await.expect("committed");
    balance
}
