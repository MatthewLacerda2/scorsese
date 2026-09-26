//! The history a user reads: one row per thing, the balance after each, in
//! reais once there is a rate, and only their own.

use scorsese_providers::prices::claude::{MODEL, Usage};
use scorsese_server::credits::generations::Answer;
use scorsese_server::credits::history::{self, Filter, History};
use scorsese_server::credits::ledger::{self, AssistantCall};
use scorsese_server::credits::rates;
use scorsese_server::db::{self, UserId};
use sqlx::postgres::PgPool;

use super::{SHOT_PRICE, account, finish, fund, shot, start};

/// A top-up of $10, a shot that worked on project 1, one that failed on
/// project 2, one still running on project 1, and an assistant turn.
pub(crate) async fn a_month_of_work(pool: &PgPool, user: UserId) {
    fund(pool, user, 10).await;
    let worked = start(pool, user, &shot(Some(1)))
        .await
        .expect("the test setup holds");
    let failed = start(pool, user, &shot(Some(2)))
        .await
        .expect("the test setup holds");
    finish(pool, user, worked, &Answer::Worked(None))
        .await
        .expect("the test setup holds");
    finish(pool, user, failed, &Answer::Failed("timeout".into()))
        .await
        .expect("the test setup holds");
    start(pool, user, &shot(Some(1)))
        .await
        .expect("the test setup holds");
    let mut tx = db::scoped(pool, user).await.expect("the test setup holds");
    let call = AssistantCall {
        model: MODEL,
        usage: Usage {
            output: 1_000,
            ..Usage::default()
        },
        project: Some(1),
        prompt: "trim the ending",
    };
    ledger::charge_assistant(&mut tx, &call)
        .await
        .expect("the test setup holds"); // 20 000 µ$ + 10%
    tx.commit().await.expect("the test setup holds");
}

/// `user`'s history, as `filter` asks.
pub(crate) async fn read(pool: &PgPool, user: UserId, filter: Filter) -> History {
    history::read(pool, user, &filter)
        .await
        .expect("the test setup holds")
}

#[sqlx::test]
async fn every_thing_is_one_row_with_the_balance_after_it(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    a_month_of_work(&pool, ana).await;
    let seen = read(&pool, ana, Filter::default()).await;

    let rows: Vec<(&str, &str, i64, i64)> = seen
        .rows
        .iter()
        .map(|row| {
            (
                row.kind.as_str(),
                row.status.as_str(),
                row.amount_micros,
                row.balance_after_micros,
            )
        })
        .collect();
    let after_shot = 10_000_000 - SHOT_PRICE;
    let after_pending = after_shot - SHOT_PRICE;
    assert_eq!(
        rows,
        [
            ("assistant", "charged", -22_000, after_pending - 22_000),
            ("veo_shot", "pending", -SHOT_PRICE, after_pending),
            ("veo_shot", "free", 0, after_shot),
            ("veo_shot", "charged", -SHOT_PRICE, after_shot),
            ("top_up", "credited", 10_000_000, 10_000_000),
        ]
    );
    assert_eq!(seen.balance_micros, after_pending - 22_000);
    assert_eq!(seen.matched, 5);
    assert_eq!(seen.total_micros, seen.balance_micros);
    assert_eq!(seen.rows[2].detail["prompt"], "a lighthouse at dusk");
    assert_eq!(seen.rows[2].detail["error"], "timeout");
    assert_eq!(seen.rows[0].detail["prompt"], "trim the ending");
    assert_eq!(seen.rows[4].detail["brl_per_usd_e4"], 50_000);
}

#[sqlx::test]
async fn amounts_are_shown_in_reais_once_a_rate_is_set(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    assert_eq!(
        read(&pool, ana, Filter::default()).await.balance_centavos,
        None
    );

    rates::set(&pool, 54_321).await.unwrap();
    let seen = read(&pool, ana, Filter::default()).await;
    assert_eq!(
        seen.balance_centavos,
        Some(5_432),
        "$10 at 5.4321 is R$ 54,32"
    );
    assert_eq!(seen.rows[0].amount_centavos, Some(5_432));
    assert_eq!(seen.rate.map(|rate| rate.brl_per_usd_e4), Some(54_321));
}

#[sqlx::test]
async fn nobody_sees_anybody_elses_history(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let bia = account(&pool, "bia@example.com").await;
    a_month_of_work(&pool, ana).await;
    let seen = read(&pool, bia, Filter::default()).await;
    assert!(seen.rows.is_empty());
    assert_eq!((seen.balance_micros, seen.matched), (0, 0));
}
