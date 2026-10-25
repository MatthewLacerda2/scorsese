//! `scorsese-server credit …`, read as the operator would read it.

use scorsese_server::ServerError;
use scorsese_server::credits::command::{CreditCommand, run};
use sqlx::postgres::PgPool;

use super::allowance::spent_free_at;
use super::{account, balance};

#[sqlx::test]
async fn a_top_up_credits_the_dollars_given(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let said = run(
        &pool,
        CreditCommand::TopUp {
            email: "ana@example.com".into(),
            dollars: "100,00".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        said,
        "credited ana@example.com $100.00; balance now $100.00"
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
    for amount in ["100.0000005", "-100", "0", "cem", ""] {
        let refused = run(
            &pool,
            CreditCommand::TopUp {
                email: "ana@example.com".into(),
                dollars: amount.into(),
            },
        )
        .await;
        assert!(
            matches!(refused, Err(ServerError::Credit(_))),
            "{amount:?}: {refused:?}"
        );
    }
    assert_eq!(balance(&pool, ana).await, 0);
}

/// What the free allowance paid, per user per Brasília day, with the month
/// to date and when each first paid (#1530).
#[sqlx::test]
async fn free_usage_says_who_spent_the_allowance_and_when(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let bia = account(&pool, "bia@example.com").await;
    super::fund(&pool, ana, 5).await;
    // 23:00 on the 2nd in Brasília, already the 3rd in UTC.
    spent_free_at(&pool, ana, "2026-10-03T02:00:00Z", 3_000_000).await;
    spent_free_at(&pool, ana, "2026-10-03T15:00:00Z", 1_000_000).await;
    spent_free_at(&pool, bia, "2026-10-03T15:00:00Z", 2_000_000).await;
    let usage = |email: Option<&str>, since: Option<&str>| CreditCommand::FreeUsage {
        email: email.map(Into::into),
        since: since.map(Into::into),
        until: None,
    };

    let all = run(&pool, usage(None, None)).await.unwrap();
    let lines: Vec<&str> = all.lines().collect();
    assert!(
        lines[0].starts_with("ana@example.com, first paid 20"),
        "{all}"
    );
    assert_eq!(
        lines[1], "  2026-10-02  $3.00 free, $3.00 this month",
        "{all}"
    );
    assert_eq!(
        lines[2], "  2026-10-03  $1.00 free, $4.00 this month",
        "{all}"
    );
    assert_eq!(lines[3], "bia@example.com, never paid", "{all}");
    assert_eq!(
        lines[4], "  2026-10-03  $2.00 free, $2.00 this month",
        "{all}"
    );

    let since = run(&pool, usage(Some("ana@example.com"), Some("2026-10-03")))
        .await
        .unwrap();
    assert_eq!(since.lines().count(), 2, "{since}");
    assert!(
        since.ends_with("  2026-10-03  $1.00 free, $4.00 this month"),
        "{since}"
    );

    let bad = run(&pool, usage(None, Some("yesterday")))
        .await
        .unwrap_err();
    assert!(bad.to_string().contains("YYYY-MM-DD"), "{bad}");
}
