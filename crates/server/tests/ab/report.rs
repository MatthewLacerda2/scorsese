//! `scorsese-server ab report`, read from what the route wrote.

use scorsese_server::ab::report::{AbCommand, run};
use scorsese_server::ab::{self, Batch, Event, Exposure, Kind, Place};
use scorsese_server::accounts::users;
use sqlx::postgres::PgPool;

use super::ANA;

fn batch(visitor: &str, version: Option<&str>, kinds: &[Kind]) -> Batch {
    Batch {
        visitor: visitor.into(),
        exposures: version
            .map(|version| Exposure {
                test: "heading".into(),
                version: version.into(),
            })
            .into_iter()
            .collect(),
        events: kinds
            .iter()
            .map(|&kind| Event {
                kind,
                value: if kind == Kind::Time { 40 } else { 0 },
            })
            .collect(),
    }
}

async fn report(pool: &PgPool) -> String {
    run(pool, AbCommand::Report { test: None })
        .await
        .expect("the report runs")
}

#[sqlx::test]
async fn the_report_counts_each_version_from_its_exposure_on(pool: PgPool) {
    scorsese_server::db::migrate(&pool).await.unwrap();
    assert_eq!(
        report(&pool).await,
        "no landing test has had a visitor yet\n"
    );

    let place = Place::new(Some("BR"), Some("SP"));
    // A prompt before the visitor was in the test does not count for it.
    ab::record(&pool, &batch(ANA, None, &[Kind::Prompt]), &place)
        .await
        .unwrap();
    let ana_saw = [Kind::View, Kind::End, Kind::Time];
    ab::record(&pool, &batch(ANA, Some("a"), &ana_saw), &place)
        .await
        .unwrap();
    let ana = users::create(&pool, "ana@padaria.com.br").await.unwrap();
    ab::link(&pool, ANA, ana).await;
    for at in 0..49 {
        let visitor = format!("1b7f6a3e-9c1d-4e2f-8a5b-{at:012x}");
        ab::record(
            &pool,
            &batch(&visitor, Some("a"), &[Kind::View]),
            &Place::default(),
        )
        .await
        .unwrap();
    }
    for at in 0..50 {
        let visitor = format!("2b7f6a3e-9c1d-4e2f-8a5b-{at:012x}");
        let kinds: &[Kind] = if at < 8 {
            &[Kind::View, Kind::Prompt]
        } else {
            &[Kind::View]
        };
        ab::record(&pool, &batch(&visitor, Some("b"), kinds), &Place::default())
            .await
            .unwrap();
    }

    let report = report(&pool).await;
    let row = |version: &str| -> Vec<String> {
        report
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("{version} ")))
            .unwrap_or_else(|| panic!("{version} in\n{report}"))
            .split_whitespace()
            .map(Into::into)
            .collect()
    };
    assert_eq!(
        row("a"),
        ["a", "50", "2%", "0", "0%", "2%", "0%", "100%"],
        "{report}"
    );
    assert_eq!(
        row("b"),
        ["b", "50", "0%", "0", "16%", "0%", "0%", "-"],
        "{report}"
    );
    assert!(
        report.contains("verdict: ready to decide: b wins on prompt rate"),
        "{report}"
    );
    assert!(report.contains("BR-SP        a 1 visitors"), "{report}");
    assert!(report.contains("unknown      a 49 visitors"), "{report}");

    let other = run(
        &pool,
        AbCommand::Report {
            test: Some("footer".into()),
        },
    )
    .await
    .unwrap();
    assert_eq!(other, "no landing test has had a visitor yet\n");
}
