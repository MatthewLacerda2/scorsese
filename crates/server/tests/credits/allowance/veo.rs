//! A Veo shot is paid from the paid balance only, in the code and in the
//! ledger's own constraints.

use scorsese_server::credits::CreditError;
use scorsese_server::credits::allowance;
use scorsese_server::credits::generations::Answer;
use scorsese_server::db;
use sqlx::postgres::PgPool;

use super::super::{SHOT_PRICE, account, balance, finish, fund, shot, start};
use super::{FOUR_A_DAY, free};

#[sqlx::test]
async fn a_veo_shot_is_never_paid_from_the_free_allowance(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    // $4 free covers a $1.44 shot, but a shot is paid only from paid credit.
    let refused = start(&pool, ana, &shot(None)).await.unwrap_err();
    assert!(
        matches!(
            refused,
            CreditError::Insufficient {
                balance: 0,
                needed: SHOT_PRICE
            }
        ),
        "{refused:?}"
    );
    // With paid credit, the shot takes all of it from there.
    fund(&pool, ana, 10).await;
    let paid = start(&pool, ana, &shot(None)).await.unwrap();
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();
    assert_eq!(balance(&pool, ana).await, 10_000_000 - SHOT_PRICE);
    assert_eq!(
        free(&pool, ana).await,
        4_000_000,
        "the day's free part untouched"
    );
}

#[sqlx::test]
async fn a_paid_balance_smaller_than_the_shot_is_refused_whatever_is_free(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 1).await;
    let refused = start(&pool, ana, &shot(None)).await.unwrap_err();
    assert!(
        matches!(
            refused,
            CreditError::Insufficient {
                balance: 1_000_000,
                needed: SHOT_PRICE
            }
        ),
        "{refused:?}"
    );
}

#[sqlx::test]
async fn the_ledger_refuses_an_allowance_entry_for_a_veo_shot(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let paid = start(&pool, ana, &shot(None)).await.unwrap();
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let refused = sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, settles, veo_generation_id,
                                     memo)
         SELECT member_id(), 'allowance', 1000, id, veo_generation_id, memo
         FROM credit_entries WHERE id = $1",
    )
    .bind(paid.reservation.entry)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("credit_entries_allowance_never_veo"),
        "{refused}"
    );
}
