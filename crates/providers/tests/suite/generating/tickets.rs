//! Tickets that will not come good the way they were bought (#1106): a shot
//! edited while in flight, a ticket the provider no longer answers for, and
//! one past the window a finished video is kept in.

use scorsese_core::{AssetId, Project, Timestamp};
use scorsese_providers::credentials::Budget;
use scorsese_providers::video::{Outcome, Plan, generate, plan};

use crate::generating::mock::{Answer, Mock};
use crate::generating::{sketched, standing};

/// One run, unlimited.
fn run(project: &mut Project, dir: &std::path::Path, provider: &Mock) -> Vec<(AssetId, Outcome)> {
    generate(project, dir, provider, Budget::unlimited(0)).expect("a run")
}

/// Points the asset's brief at `prompt`.
fn reprompt(project: &mut Project, id: &AssetId, prompt: &str) {
    let asset = project.assets.iter_mut().find(|a| &a.id == id);
    asset.expect("the asset").prompt = Some(prompt.to_owned());
}

/// The bug: the old prompt's video used to be filed under the new prompt's
/// name, and the new prompt was then never sent.
#[test]
fn a_shot_edited_in_flight_files_its_video_under_the_brief_that_bought_it() {
    let (dir, mut project, id) = sketched("edited-in-flight", "a red balloon at dusk");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"OLD TAKE".to_vec())]);
    run(&mut project, &dir, &provider);

    reprompt(&mut project, &id, "a blue kite at noon");
    let arrived = run(&mut project, &dir, &provider);
    let Outcome::Failed { message } = &arrived[0].1 else {
        panic!("expected the shot released, got {:?}", arrived[0].1);
    };
    assert!(message.contains("brief changed"), "{message}");
    assert_eq!(
        standing(&project, &id),
        (String::from("Some(Sketch)"), None, None)
    );
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(
        plan(&project, &dir, asset),
        Plan::Submit,
        "the new brief is owed"
    );

    let again = run(&mut project, &dir, &provider);
    assert!(
        matches!(again[0].1, Outcome::Queued { .. }),
        "{:?}",
        again[0].1
    );
    assert_eq!(provider.submissions(), 2, "the new brief is sent once");

    reprompt(&mut project, &id, "a red balloon at dusk");
    let asset = project.asset(&id).expect("the asset");
    let Plan::Realized(path) = plan(&project, &dir, asset) else {
        panic!("the old brief's video was paid for and kept");
    };
    assert_eq!(
        std::fs::read(path.resolve(&dir)).ok(),
        Some(b"OLD TAKE".to_vec())
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A ticket from before the sidecar existed lands where it always did.
#[test]
fn a_ticket_with_nothing_remembered_lands_under_the_current_brief() {
    let (dir, mut project, id) = sketched("unremembered", "a pit lane in 1976");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"MP4".to_vec())]);
    run(&mut project, &dir, &provider);
    std::fs::remove_dir_all(dir.join("generated/tickets")).expect("a sidecar was written");

    let arrived = run(&mut project, &dir, &provider);
    assert!(
        matches!(arrived[0].1, Outcome::Generated { .. }),
        "{:?}",
        arrived[0].1
    );
    assert_eq!(standing(&project, &id).0, "Some(Generated)");
    std::fs::remove_dir_all(&dir).ok();
}

/// A 404 on one ticket used to stop every run in the project, stills and
/// lines included, for ever.
#[test]
fn a_lost_ticket_is_cleared_as_its_own_shot_and_the_run_goes_on() {
    let (dir, mut project, id) = sketched("lost", "a harbour at dawn");
    let other = AssetId::new("later");
    project.assets.push(scorsese_core::Asset::sketch(
        other.clone(),
        scorsese_core::AssetKind::GeneratedVideo,
        "a lighthouse",
    ));
    let provider = Mock::answering("shot", vec![Answer::Lost]);
    run(&mut project, &dir, &provider);

    let next = run(&mut project, &dir, &provider);
    let Outcome::Failed { message } = &next[0].1 else {
        panic!("expected the ticket cleared, got {:?}", next[0].1);
    };
    assert!(message.contains("404"), "{message}");
    assert_eq!(
        standing(&project, &id),
        (String::from("Some(Sketch)"), None, None)
    );
    assert!(
        matches!(next[1].1, Outcome::Waiting { .. }),
        "the run went on: {next:?}"
    );
    assert!(!dir.join("generated/tickets/shot").exists());
    std::fs::remove_dir_all(&dir).ok();
}

/// Past the window the video is gone; the shot is released so it can be
/// asked for again, which nothing could do while the ticket stood.
#[test]
fn an_expired_ticket_is_released_and_the_next_run_sends_it_again() {
    let (dir, mut project, id) = sketched("expired", "a desert road");
    let provider = Mock::default();
    run(&mut project, &dir, &provider);
    let asset = project
        .assets
        .iter_mut()
        .find(|a| a.id == id)
        .expect("the asset");
    asset.queued_at = Timestamp::from_utc(2020, 1, 1, 0, 0, 0).ok();

    let polled = run(&mut project, &dir, &provider);
    assert!(
        matches!(polled[0].1, Outcome::Expired { .. }),
        "{:?}",
        polled[0].1
    );
    assert_eq!(
        standing(&project, &id),
        (String::from("Some(Sketch)"), None, None)
    );

    let again = run(&mut project, &dir, &provider);
    assert!(
        matches!(again[0].1, Outcome::Queued { .. }),
        "{:?}",
        again[0].1
    );
    assert_eq!(provider.submissions(), 2);
    std::fs::remove_dir_all(&dir).ok();
}
