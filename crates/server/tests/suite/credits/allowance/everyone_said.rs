//! What a spend is told once everyone's free ceiling (#1658) is spent.

use scorsese_server::credits::{CreditError, allowance};
use scorsese_server::db;
use sqlx::postgres::PgPool;

use super::super::{account, shot, start};
use super::{FOUR_A_DAY, free_at};

/// Every account $4 a day, and nothing for all of them together.
async fn nothing_for_everyone(pool: &PgPool) {
    let mut none = FOUR_A_DAY;
    none.total_daily_micros = 0;
    allowance::set(pool, none)
        .await
        .expect("the allowance is set");
}

/// A Veo shot is still told free credit never pays for one, though none is
/// left for anyone today: tomorrow's would not pay for it either.
#[sqlx::test]
async fn a_shot_still_says_free_credit_never_pays_for_it(pool: PgPool) {
    nothing_for_everyone(&pool).await;
    let ana = account(&pool, "ana@example.com").await;
    let refused = start(&pool, ana, &shot(None)).await.unwrap_err();
    assert!(
        matches!(refused, CreditError::PaidOnly { paid: 0, .. }),
        "{refused:?}"
    );
}

/// Nothing renews when everyone together is given nothing.
#[sqlx::test]
async fn a_ceiling_of_nothing_renews_nothing(pool: PgPool) {
    nothing_for_everyone(&pool).await;
    let ana = account(&pool, "ana@example.com").await;
    let now = free_at(&pool, ana, 1_793_000_000).await;
    assert_eq!(now.left_micros, 0);
    assert_eq!(now.renews_at, 0);
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    assert!(allowance::used_up_for_everyone(&mut tx).await.unwrap());
}
