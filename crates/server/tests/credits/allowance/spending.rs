//! Free first, then paid: a generation and an assistant call.

use scorsese_providers::chat::{Model, Usage};
use scorsese_server::credits::generations::Answer;
use scorsese_server::credits::ledger::{self, AssistantCall};
use scorsese_server::credits::{CreditError, Product, price};
use scorsese_server::credits::{allowance, history, tool};
use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::super::{account, balance, finish, fund, start};
use super::{FOUR_A_DAY, free, line};

#[sqlx::test]
async fn a_server_that_gives_nothing_away_leaves_a_new_account_at_zero(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let settings = json!({});
    assert_eq!(free(&pool, ana).await, 0);
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let none = allowance::left(&mut tx).await.unwrap();
    assert_eq!(none.renews_at, 0, "nothing given, nothing renews");
    drop(tx);
    let refused = start(&pool, ana, &line(10, &settings)).await.unwrap_err();
    assert!(
        matches!(refused, CreditError::Insufficient { balance: 0, .. }),
        "{refused:?}"
    );
}

#[sqlx::test]
async fn a_new_account_spends_its_free_allowance_and_is_stopped_after(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    let settings = json!({});
    assert_eq!(free(&pool, ana).await, 4_000_000);

    // $1.80 and $1.80: both covered, all of it free.
    for _ in 0..2 {
        let paid = start(&pool, ana, &line(100, &settings)).await.unwrap();
        finish(&pool, ana, paid, &Answer::Worked(None))
            .await
            .unwrap();
    }
    assert_eq!(balance(&pool, ana).await, 0, "nothing paid, nothing owed");
    assert_eq!(free(&pool, ana).await, 400_000);
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    assert_eq!(ledger::spendable(&mut tx).await.unwrap(), 400_000);
    drop(tx);

    // Another $1.80 is more than the $0.40 left.
    let refused = start(&pool, ana, &line(100, &settings)).await.unwrap_err();
    assert!(
        matches!(
            refused,
            CreditError::Insufficient {
                balance: 400_000,
                needed: 1_800_000
            }
        ),
        "{refused:?}"
    );
}

#[sqlx::test]
async fn the_free_part_is_spent_first_and_the_paid_balance_takes_the_rest(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let settings = json!({});
    // $5.40: $4 free, $1.40 paid.
    let paid = start(&pool, ana, &line(300, &settings)).await.unwrap();
    assert_eq!(balance(&pool, ana).await, 10_000_000 - 1_400_000);
    assert_eq!(free(&pool, ana).await, 0);
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();
    assert_eq!(balance(&pool, ana).await, 10_000_000 - 1_400_000);
    assert_eq!(free(&pool, ana).await, 0);
}

#[sqlx::test]
async fn an_assistant_call_is_paid_from_the_allowance_first(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    let call = AssistantCall {
        model: Model::ClaudeOpus55.id(),
        usage: Usage {
            input: 2_000,
            output: 600,
            cache_read: 50_000,
            ..Usage::default()
        },
        project: Some(3),
        prompt: "cut the intro",
        turn: None,
    };
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let charged = ledger::charge_assistant(&mut tx, &call).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(charged.micros, price(Product::Assistant, 30_000));
    assert_eq!(balance(&pool, ana).await, 0, "the allowance paid all of it");
    assert_eq!(free(&pool, ana).await, 4_000_000 - charged.micros);
}

/// The assistant never learns the split (#1530): the history tool says what
/// can be spent and what each thing cost, and a total counts the free part.
#[sqlx::test]
async fn the_history_tool_says_what_can_be_spent_and_what_things_cost(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    let settings = json!({});
    let paid = start(&pool, ana, &line(100, &settings)).await.unwrap();
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();
    let seen = history::read(&pool, ana, &Default::default())
        .await
        .unwrap();
    assert_eq!(seen.total_micros, -1_800_000, "the free part is spent too");
    assert_eq!(seen.free_micros, 2_200_000);
    let text = tool::answer(&seen);
    assert!(text.starts_with("Balance: $2.20.\n"), "{text}");
    assert!(text.contains("adding up to -$1.80"), "{text}");
    assert!(text.contains(": -$1.80"), "{text}");
    assert!(!text.contains("free"), "{text}");
}
