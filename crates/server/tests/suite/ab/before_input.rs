//! Visitors stored before the landing sent `input` (#1797), as migration
//! `0054` judged them and the report counts them.

use scorsese_server::ab::{self, Kind, Place};
use sqlx::postgres::PgPool;

use super::report::{batch, report};

/// A visitor stored before the page sent `input`, at `place`, that did `kinds`.
async fn before_input(pool: &PgPool, code: &str, place: (&str, &str), kinds: &[(&str, i32)]) {
    sqlx::query("INSERT INTO ab_visitors (code, country, region) VALUES ($1::uuid, $2, $3)")
        .bind(code)
        .bind(place.0)
        .bind(place.1)
        .execute(pool)
        .await
        .expect("the visitor is stored");
    sqlx::query(
        "INSERT INTO ab_exposures (visitor, test, version) VALUES ($1::uuid, 'heading', 'a')",
    )
    .bind(code)
    .execute(pool)
    .await
    .expect("the exposure is stored");
    for (kind, value) in kinds {
        sqlx::query("INSERT INTO ab_events (visitor, kind, value) VALUES ($1::uuid, $2, $3)")
            .bind(code)
            .bind(kind)
            .bind(value)
            .execute(pool)
            .await
            .expect("the event is stored");
    }
}

#[sqlx::test(migrations = false)]
async fn visitors_stored_before_input_are_judged_by_where_and_how_far(pool: PgPool) {
    // The schema as it was before the page sent `input` (#1797).
    scorsese_server::db::MIGRATOR
        .run_to(53, &pool)
        .await
        .unwrap();
    let view = [("view", 0), ("time", 25)];
    let scrolled = [("view", 0), ("section", 2), ("time", 90)];
    // The datacenter loads: Virginia and Iowa, never past the first screen.
    before_input(
        &pool,
        "4b7f6a3e-9c1d-4e2f-8a5b-000000000001",
        ("US", "VA"),
        &view,
    )
    .await;
    before_input(
        &pool,
        "4b7f6a3e-9c1d-4e2f-8a5b-000000000002",
        ("US", "IA"),
        &view,
    )
    .await;
    // People: one from Virginia who scrolled, and anyone from elsewhere.
    before_input(
        &pool,
        "4b7f6a3e-9c1d-4e2f-8a5b-000000000003",
        ("US", "VA"),
        &scrolled,
    )
    .await;
    before_input(
        &pool,
        "4b7f6a3e-9c1d-4e2f-8a5b-000000000004",
        ("BR", "SP"),
        &view,
    )
    .await;
    before_input(
        &pool,
        "4b7f6a3e-9c1d-4e2f-8a5b-000000000005",
        ("US", "CA"),
        &view,
    )
    .await;
    scorsese_server::db::migrate(&pool).await.unwrap();
    // A browser first seen since is judged by its input alone, wherever it is.
    ab::record(
        &pool,
        &batch(
            "4b7f6a3e-9c1d-4e2f-8a5b-000000000006",
            Some("a"),
            &[Kind::View],
        ),
        &Place::new(Some("BR"), Some("SP")),
    )
    .await
    .unwrap();

    let report = report(&pool).await;
    assert!(report.contains("  left out: 3 loads"), "{report}");
    assert!(report.contains("US-VA        a 1 visitors"), "{report}");
    assert!(!report.contains("US-IA"), "{report}");
    assert!(report.contains("BR-SP        a 1 visitors"), "{report}");
}
