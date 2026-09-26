//! Filtering the history: by project, kind and days, paged, with a total
//! over everything matched — and a filter that does not read is refused.

use scorsese_server::credits::history::{self, Filter};
use sqlx::postgres::PgPool;

use super::history::{a_month_of_work, read};
use super::{SHOT_PRICE, account};

#[sqlx::test]
async fn a_filter_narrows_the_rows_and_totals_what_it_matched(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    a_month_of_work(&pool, ana).await;

    let project = read(
        &pool,
        ana,
        Filter {
            project: Some(1),
            ..Filter::default()
        },
    )
    .await;
    assert_eq!(project.matched, 3);
    assert_eq!(project.total_micros, -2 * SHOT_PRICE - 22_000);

    let shots = read(
        &pool,
        ana,
        Filter {
            kind: Some("veo_shot".into()),
            limit: Some(1),
            ..Filter::default()
        },
    )
    .await;
    assert_eq!(
        (shots.matched, shots.rows.len()),
        (3, 1),
        "the total covers every page"
    );
    let next = read(
        &pool,
        ana,
        Filter {
            kind: Some("veo_shot".into()),
            before: Some(shots.rows[0].id),
            ..Filter::default()
        },
    )
    .await;
    assert_eq!(next.rows.len(), 2);

    let future = read(
        &pool,
        ana,
        Filter {
            since: Some("2999-01-01".into()),
            ..Filter::default()
        },
    )
    .await;
    assert_eq!((future.matched, future.total_micros), (0, 0));
    let past = read(
        &pool,
        ana,
        Filter {
            until: Some("2000-01-01".into()),
            ..Filter::default()
        },
    )
    .await;
    assert!(past.rows.is_empty());
}

#[sqlx::test]
async fn a_filter_that_does_not_parse_is_refused_in_words(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    for filter in [
        Filter {
            kind: Some("everything".into()),
            ..Filter::default()
        },
        Filter {
            since: Some("last week".into()),
            ..Filter::default()
        },
    ] {
        let error = history::read(&pool, ana, &filter).await.unwrap_err();
        assert!(!error.to_string().contains("database"), "{error}");
    }
}
