//! `scorsese-server credit adjust`: the operator taking test money back
//! (#1542).

use scorsese_server::ServerError;
use scorsese_server::credits::command::{CreditCommand, run};
use scorsese_server::credits::history::Filter;
use scorsese_server::db::{self, UserId};
use sqlx::postgres::PgPool;

use super::{account, balance};

/// `credit adjust`, as the operator would type it.
fn adjust(dollars: Option<&str>, to_zero: bool, include_paid: bool) -> CreditCommand {
    CreditCommand::Adjust {
        email: "ana@example.com".into(),
        dollars: dollars.map(Into::into),
        to_zero,
        reason: "test credit taken back".into(),
        include_paid,
    }
}

/// A Stripe payment of `user`'s that arrived, as the webhook leaves it.
async fn paid_payment(pool: &PgPool, user: UserId) {
    let mut tx = db::privileged(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO payments (user_id, stripe_session, brl_centavos, credit_micros,
                               brl_per_usd, rate_source, state, paid_at)
         VALUES ($1, 'cs_test', 5000, 10000000, '5.0', 'awesomeapi', 'paid', now())",
    )
    .bind(user.get())
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// Taking test money back is one new negative entry, with its reason, that
/// the user's history shows as its own line (#1542).
#[sqlx::test]
async fn an_adjustment_takes_money_back_as_one_new_entry(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    super::fund(&pool, ana, 10).await;

    let said = run(&pool, adjust(Some("-2.50"), false, false))
        .await
        .unwrap();
    assert_eq!(
        said,
        "took $2.50 back from ana@example.com; balance now $7.50"
    );
    let said = run(&pool, adjust(None, true, false)).await.unwrap();
    assert_eq!(
        said,
        "took $7.50 back from ana@example.com; balance now $0.00"
    );
    assert_eq!(balance(&pool, ana).await, 0);

    let seen = super::history::read(&pool, ana, Default::default()).await;
    let rows: Vec<(&str, &str, &str, i64)> = seen
        .rows
        .iter()
        .map(|row| {
            (
                row.kind.as_str(),
                row.status.as_str(),
                row.memo.as_str(),
                row.amount_micros,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                "adjustment",
                "charged",
                "Adjustment: test credit taken back",
                -7_500_000
            ),
            (
                "adjustment",
                "charged",
                "Adjustment: test credit taken back",
                -2_500_000
            ),
            ("top_up", "credited", "Top-up", 10_000_000),
        ]
    );
    let only = super::history::read(
        &pool,
        ana,
        Filter {
            kind: Some("adjustment".into()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(only.matched, 2);
}

/// An adjustment only takes money down, never past zero, and a balance at
/// zero is left alone with nothing written.
#[sqlx::test]
async fn an_adjustment_never_adds_or_goes_below_zero(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let said = run(&pool, adjust(None, true, false)).await.unwrap();
    assert_eq!(said, "ana@example.com is already at $0.00; nothing written");
    super::fund(&pool, ana, 5).await;
    for dollars in ["2", "+2", "-5.01", "-0", "-cem"] {
        let refused = run(&pool, adjust(Some(dollars), false, false)).await;
        assert!(
            matches!(refused, Err(ServerError::Credit(_))),
            "{dollars:?}: {refused:?}"
        );
    }
    for (dollars, to_zero) in [(None, false), (Some("-1"), true)] {
        let refused = run(&pool, adjust(dollars, to_zero, false)).await;
        assert!(refused.is_err(), "{dollars:?} {to_zero}: {refused:?}");
    }
    let blank = run(
        &pool,
        CreditCommand::Adjust {
            email: "ana@example.com".into(),
            dollars: None,
            to_zero: true,
            reason: "  ".into(),
            include_paid: false,
        },
    )
    .await;
    assert!(blank.is_err(), "a reason is required: {blank:?}");
    assert_eq!(balance(&pool, ana).await, 5_000_000);
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
