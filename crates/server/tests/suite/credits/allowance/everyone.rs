//! The ceiling on free spend for everyone together (#1658): every account's
//! allowance is drawn from one operator-wide daily pot, so the number of
//! accounts cannot multiply what the operator gives away.

use std::time::Duration;

use scorsese_providers::chat::{Model, Usage};
use scorsese_server::credits::allowance;
use scorsese_server::credits::generations::{self, Answer};
use scorsese_server::credits::ledger::{self, AssistantCall};
use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::{account, balance, finish, fund, start};
use super::{FOUR_A_DAY, epoch, free, free_at, line, spent_free_at};

/// Give every account $4 a day, all of them together at most `dollars` a day.
async fn ceiling(pool: &PgPool, dollars: i64) {
    allowance::set(pool, FOUR_A_DAY).await.unwrap();
    sqlx::query("UPDATE free_allowance SET total_daily_micros = $1")
        .bind(dollars * 1_000_000)
        .execute(pool)
        .await
        .expect("the ceiling is a column of free_allowance");
}

/// What the refusal says once everyone's free credit is spent.
const USED_UP: &str = "today's free credit for everyone is used up";

#[sqlx::test]
async fn the_second_member_is_refused_past_the_ceiling_and_paid_credit_still_spends(pool: PgPool) {
    ceiling(&pool, 5).await;
    let ana = account(&pool, "ana@example.com").await;
    let bea = account(&pool, "bea@example.com").await;
    let settings = json!({});

    // Ana spends $3.60 of her $4, all of it free.
    for _ in 0..2 {
        let paid = start(&pool, ana, &line(100, &settings)).await.unwrap();
        finish(&pool, ana, paid, &Answer::Worked(None))
            .await
            .unwrap();
    }
    assert_eq!(balance(&pool, ana).await, 0);
    assert_eq!(free(&pool, bea).await, 1_400_000, "$5 less Ana's $3.60");

    // Bea's own $4 would cover $1.80, but only $1.40 is left for everyone.
    let refused = start(&pool, bea, &line(100, &settings)).await.unwrap_err();
    let said = refused.to_string();
    assert!(said.contains(USED_UP), "{said}");
    assert!(said.contains("tomorrow"), "{said}");
    assert_eq!(free(&pool, bea).await, 1_400_000, "nothing was granted");

    // With paid credit she goes ahead: the $1.40 free, then $0.40 paid.
    fund(&pool, bea, 10).await;
    let paid = start(&pool, bea, &line(100, &settings)).await.unwrap();
    finish(&pool, bea, paid, &Answer::Worked(None))
        .await
        .unwrap();
    assert_eq!(balance(&pool, bea).await, 10_000_000 - 400_000);
    assert_eq!(
        free(&pool, ana).await,
        0,
        "the ceiling is spent for everyone"
    );

    // And past it, the paid balance pays all of it.
    let paid = start(&pool, bea, &line(100, &settings)).await.unwrap();
    finish(&pool, bea, paid, &Answer::Worked(None))
        .await
        .unwrap();
    assert_eq!(balance(&pool, bea).await, 10_000_000 - 400_000 - 1_800_000);
}

/// An assistant call is never refused, the tokens being spent; past the
/// ceiling it is paid from the paid balance.
#[sqlx::test]
async fn an_assistant_call_past_the_ceiling_is_not_paid_free(pool: PgPool) {
    ceiling(&pool, 0).await;
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 1).await;
    let call = AssistantCall {
        model: Model::ClaudeOpus55.id(),
        usage: Usage {
            input: 2_000,
            output: 600,
            ..Usage::default()
        },
        project: None,
        prompt: "cut the intro",
        turn: None,
    };
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let charged = ledger::charge_assistant(&mut tx, &call).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(balance(&pool, ana).await, 1_000_000 - charged.micros);
}

/// The ceiling turns with the Brasília day, like each account's own.
#[sqlx::test]
async fn the_next_day_free_spend_resumes(pool: PgPool) {
    ceiling(&pool, 5).await;
    let ana = account(&pool, "ana@example.com").await;
    let bea = account(&pool, "bea@example.com").await;
    let cris = account(&pool, "cris@example.com").await;
    spent_free_at(&pool, ana, "2026-10-20T15:00:00Z", 3_000_000).await;
    spent_free_at(&pool, bea, "2026-10-20T16:00:00Z", 2_000_000).await;

    let today = free_at(&pool, cris, epoch(&pool, "2026-10-21T02:59:59Z").await).await;
    assert_eq!(today.left_micros, 0, "everyone's $5 is spent");
    assert_eq!(today.renews_at, epoch(&pool, "2026-10-21T03:00:00Z").await);
    let tomorrow = free_at(&pool, cris, epoch(&pool, "2026-10-21T03:00:00Z").await).await;
    assert_eq!(tomorrow.left_micros, 4_000_000);
    let ana_tomorrow = free_at(&pool, ana, epoch(&pool, "2026-10-21T03:00:00Z").await).await;
    assert_eq!(ana_tomorrow.left_micros, 4_000_000);
}

/// The check and the grant are one decision: a second member's spend waits
/// for the first to commit, then sees what it took.
#[sqlx::test]
async fn two_grants_at_once_that_together_cross_it_only_one_passes(pool: PgPool) {
    ceiling(&pool, 2).await;
    let ana = account(&pool, "ana@example.com").await;
    let bea = account(&pool, "bea@example.com").await;

    // Ana's $1.80 is reserved and granted, not yet committed.
    let ana_settings = json!({});
    let mut first = db::scoped(&pool, ana).await.unwrap();
    generations::start(&mut first, &line(100, &ana_settings))
        .await
        .unwrap();

    let second = tokio::spawn({
        let pool = pool.clone();
        async move {
            let settings = json!({});
            start(&pool, bea, &line(100, &settings)).await
        }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !second.is_finished(),
        "Bea's grant was decided while Ana's was still open"
    );
    first.commit().await.unwrap();

    let refused = second.await.unwrap().unwrap_err();
    assert!(refused.to_string().contains(USED_UP), "{refused}");
    assert_eq!(free(&pool, bea).await, 200_000, "$2 less Ana's $1.80");
}
