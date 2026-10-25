//! What `credit adjust` refuses to take back (#1542): a Stripe purchase,
//! and a reservation a running generation still holds.

use scorsese_server::credits::command::run;
use scorsese_server::credits::generations::Answer;
use scorsese_server::db::{self, UserId};
use sqlx::postgres::PgPool;

use super::adjust::adjust;
use super::{account, balance};

/// A Stripe payment of `user`'s that arrived, as the webhook leaves it.
async fn paid_payment(pool: &PgPool, user: UserId) {
    let mut tx = db::privileged(pool)
        .await
        .expect("a privileged scope opens");
    sqlx::query(
        "INSERT INTO payments (user_id, stripe_session, brl_centavos, credit_micros,
                               brl_per_usd, rate_source, state, paid_at)
         VALUES ($1, 'cs_test', 5000, 10000000, '5.0', 'awesomeapi', 'paid', now())",
    )
    .bind(user.get())
    .execute(&mut *tx)
    .await
    .expect("a paid payment is recorded");
    tx.commit().await.expect("the payment commits");
}

/// Money a user paid through Stripe is never taken back by accident: only
/// `--include-paid` lets an adjustment touch such an account.
#[sqlx::test]
async fn an_adjustment_refuses_an_account_that_paid_unless_told(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    super::fund(&pool, ana, 10).await;
    paid_payment(&pool, ana).await;

    let refused = run(&pool, adjust(None, true, false)).await.unwrap_err();
    assert!(refused.to_string().contains("--include-paid"), "{refused}");
    assert_eq!(balance(&pool, ana).await, 10_000_000);

    let said = run(&pool, adjust(None, true, true)).await.unwrap();
    assert!(said.ends_with("balance now $0.00"), "{said}");
}

/// A reset waits for a generation still running: its reservation would come
/// back if it failed, after the balance was said to be zero.
#[sqlx::test]
async fn a_reset_waits_for_a_running_generation(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    super::fund(&pool, ana, 10).await;
    let paid = super::start(&pool, ana, &super::shot(None))
        .await
        .expect("ten dollars cover a shot");

    let refused = run(&pool, adjust(None, true, false)).await.unwrap_err();
    assert!(refused.to_string().contains("still running"), "{refused}");
    assert_eq!(balance(&pool, ana).await, 10_000_000 - super::SHOT_PRICE);

    super::finish(&pool, ana, paid, &Answer::Failed("timeout".into()))
        .await
        .expect("the failure settles");
    let said = run(&pool, adjust(None, true, false)).await.unwrap();
    assert_eq!(
        said,
        "took $10.00 back from ana@example.com; balance now $0.00"
    );
}
