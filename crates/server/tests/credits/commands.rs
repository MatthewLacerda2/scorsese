//! `scorsese-server credit …`, read as the operator would read it.

use scorsese_server::ServerError;
use scorsese_server::credits::command::{CreditCommand, run};
use sqlx::postgres::PgPool;

use super::{account, balance};

#[sqlx::test]
async fn a_top_up_credits_the_reais_at_the_rate_given(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let said = run(
        &pool,
        CreditCommand::TopUp {
            email: "ana@example.com".into(),
            reais: "543,21".into(),
            rate: "5.4321".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        said,
        "credited ana@example.com $100.00 for R$ 543,21 at 5.4321; balance now $100.00"
    );
    assert_eq!(balance(&pool, ana).await, 100_000_000);

    let refunded = run(
        &pool,
        CreditCommand::Refund {
            email: "ana@example.com".into(),
            dollars: "1.06".into(),
            reason: "the shot was the wrong way up".into(),
        },
    )
    .await
    .unwrap();
    assert!(refunded.ends_with("balance now $101.06"), "{refunded}");

    let read = run(
        &pool,
        CreditCommand::Balance {
            email: "ana@example.com".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(read, "ana@example.com: $101.06");
}

/// A figure about money that does not read exactly is refused, not guessed.
#[sqlx::test]
async fn a_figure_that_does_not_parse_is_refused(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    for (reais, rate) in [
        ("100.505", "5.43"),
        ("-100", "5.43"),
        ("100", "0"),
        ("cem", "5"),
    ] {
        let refused = run(
            &pool,
            CreditCommand::TopUp {
                email: "ana@example.com".into(),
                reais: reais.into(),
                rate: rate.into(),
            },
        )
        .await;
        assert!(
            matches!(refused, Err(ServerError::Credit(_))),
            "{reais} {rate}: {refused:?}"
        );
    }
    assert_eq!(balance(&pool, ana).await, 0);
}

#[sqlx::test]
async fn the_display_rate_is_set_by_the_operator(pool: PgPool) {
    let said = run(
        &pool,
        CreditCommand::Rate {
            brl_per_usd: "5.43".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(said, "balances are now shown at 5.4300 reais per dollar");
}
