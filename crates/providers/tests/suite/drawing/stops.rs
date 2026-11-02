//! A run of stills that cannot do everything: where it stops, and that what it
//! drew before stopping is reported, not only the reason (#1153).

use scorsese_core::{Asset, AssetId, AssetKind};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{ImageError, Outcome, Stopped, generate};

use crate::drawing::mock::Mock;
use crate::drawing::sketched;

/// A ceiling that admits two stills of three stops the run at the third, and
/// the error carries the two it drew — paid for, and recorded.
#[test]
fn a_run_stopped_by_the_ceiling_reports_the_stills_it_drew() {
    let (dir, mut project, _) = sketched("stills-stopped", "a lighthouse");
    for id in ["harbour", "storm"] {
        project.assets.push(Asset::sketch(
            AssetId::new(id),
            AssetKind::GeneratedImage,
            "another still",
        ));
    }
    let provider = Mock::willing();

    // A default still is 6 cents: 12 admits two and cannot admit three.
    let stopped = generate(&mut project, &dir, &provider, Budget::new(12, 0))
        .expect_err("three stills at 6 cents is over a 12-cent ceiling");

    let drawn: Vec<&str> = stopped.run.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(drawn, ["poster", "harbour"]);
    assert!(
        stopped
            .run
            .iter()
            .all(|(_, outcome)| matches!(outcome, Outcome::Generated { .. })),
        "{:?}",
        stopped.run
    );
    let unsent: Vec<&str> = stopped.unsent.iter().map(AssetId::as_str).collect();
    assert_eq!(unsent, ["storm"]);
    assert_eq!(provider.requests(), 2, "nothing after the stop was tried");

    let says = stopped.says();
    assert!(
        says.contains("2 stills were paid for, 1 still was not (storm)"),
        "{says}"
    );
    assert!(says.contains("ceiling"), "the reason is kept: {says}");
    assert!(stopped.to_string().contains("ceiling"), "{stopped}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A run stopped before it drew anything carries nothing, and names every
/// still it would have sent.
#[test]
fn a_run_stopped_at_the_first_still_carries_nothing() {
    let (dir, mut project, _) = sketched("stills-none", "a lighthouse");
    let provider = Mock::willing();

    let stopped = generate(&mut project, &dir, &provider, Budget::new(1, 0))
        .expect_err("a 6-cent still is over a 1-cent ceiling");

    assert!(stopped.run.is_empty(), "{:?}", stopped.run);
    let unsent: Vec<&str> = stopped.unsent.iter().map(AssetId::as_str).collect();
    assert_eq!(unsent, ["poster"]);
    assert!(
        stopped
            .says()
            .contains("0 stills were paid for, 1 still was not"),
        "{}",
        stopped.says()
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Only what was drawn or ordered is counted as paid for and kept; a billed
/// refusal is named apart, and neither a cached still nor an unbilled
/// refusal is either (#1695).
#[test]
fn a_stopped_run_names_only_its_billed_refusals_apart() {
    let failed = |cents| Outcome::Failed {
        message: String::from("no"),
        estimated_cost_cents: cents,
    };
    let path = scorsese_core::ProjectPath::new("generated/still-abc.jpg");
    let stopped = Stopped {
        run: vec![
            (
                AssetId::new("cached"),
                Outcome::Cached { path: path.clone() },
            ),
            (
                AssetId::new("drawn"),
                Outcome::Generated {
                    path,
                    bytes: 3,
                    estimated_cost_cents: 6,
                },
            ),
            (AssetId::new("free"), failed(0)),
            (AssetId::new("billed"), failed(6)),
        ],
        unsent: Vec::new(),
        error: ImageError::NoSuchAsset {
            id: AssetId::new("gone"),
        },
    };

    let says = stopped.says();
    assert!(says.contains("1 still was paid for"), "{says}");
    assert!(
        says.contains("1 still was refused and billed anyway (billed);"),
        "{says}"
    );
}
