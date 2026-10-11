//! A run that cannot do everything: what it skips, where it stops, and that
//! what it sent before stopping is never lost (#1057).

use scorsese_core::{Asset, AssetId, AssetKind, Project, VideoRequest};
use scorsese_providers::credentials::Budget;
use scorsese_providers::video::{Outcome, generate};

use crate::mock::Mock;
use crate::{sketched, standing};

/// Adds a sketched shot called `id`.
fn shot(project: &mut Project, id: &str) {
    project.assets.push(Asset::sketch(
        AssetId::new(id),
        AssetKind::GeneratedVideo,
        "another shot",
    ));
}

#[test]
fn a_run_stopped_at_the_third_shot_reports_the_first_two_as_sent() {
    let (dir, mut project, _) = sketched("stopped", "the first shot");
    for id in ["second", "third", "fourth"] {
        shot(&mut project, id);
    }
    let provider = Mock::default();
    *provider.limit.borrow_mut() = Some(2);

    let stopped = generate(&mut project, &dir, &provider, Budget::unlimited(0))
        .expect_err("the third submission is refused");

    let sent: Vec<&str> = stopped
        .run
        .outcomes
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(sent, ["shot", "second"]);
    assert!(
        stopped
            .run
            .outcomes
            .iter()
            .all(|(_, outcome)| matches!(outcome, Outcome::Queued { .. }))
    );
    assert_eq!(stopped.run.spent_cents, 2 * 96, "two shots were paid for");
    let unsent: Vec<&str> = stopped.unsent.iter().map(AssetId::as_str).collect();
    assert_eq!(
        unsent,
        ["third", "fourth"],
        "nothing after the stop was tried"
    );
    assert_eq!(provider.submissions(), 2);

    // The tickets of what was sent are in the document, and nothing else is.
    let ticket = |id: &str| standing(&project, &AssetId::new(id)).2;
    assert_eq!(ticket("shot").as_deref(), Some("operations/shot"));
    assert_eq!(ticket("second").as_deref(), Some("operations/second"));
    assert_eq!(ticket("third"), None);

    let says = stopped.says();
    assert!(
        says.contains("2 shots were sent, 2 shots were not"),
        "{says}"
    );
    assert!(says.contains("third, fourth"), "{says}");
    assert!(
        says.contains("429"),
        "the provider's reason is kept: {says}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_shot_waiting_on_a_still_is_skipped_and_the_rest_are_sent() {
    let (dir, mut project, id) = sketched("waiting", "opens on the still");
    project.assets.push(Asset::sketch(
        AssetId::new("still"),
        AssetKind::GeneratedImage,
        "the opening still",
    ));
    let asset = project.assets.iter_mut().find(|asset| asset.id == id);
    asset.expect("the shot").video = Some(VideoRequest {
        first_image: Some(AssetId::new("still")),
        ..VideoRequest::default()
    });
    shot(&mut project, "second");
    let provider = Mock::default();

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0))
        .expect("one shot's missing still does not stop the run");

    let Outcome::Unready { why } = &run[0].1 else {
        panic!("expected the shot to wait, got {:?}", run[0].1);
    };
    assert!(why.contains("has not been generated yet"), "{why}");
    assert!(run[0].1.says().starts_with("not sent"));
    assert!(matches!(run[1].1, Outcome::Queued { .. }), "{:?}", run[1].1);
    assert_eq!(*provider.submitted.borrow(), ["second"]);
    assert_eq!(standing(&project, &id).2, None, "nothing was sent for it");
    std::fs::remove_dir_all(&dir).ok();
}
