//! sketch → queued → generated, and what happens when it goes wrong.

use scorsese_providers::credentials::Budget;
use scorsese_providers::video::{Outcome, collect, generate};

use crate::mock::{Answer, Mock};
use crate::{sketched, standing};

/// The ordinary path, in the two passes it really takes: the first hands the
/// brief over and writes the ticket down, the second collects the video.
#[test]
fn a_sketch_is_queued_then_generated() {
    let (dir, mut project, id) = sketched("lifecycle", "a red balloon at dusk");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"MP4 BYTES".to_vec())]);

    let first = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert!(
        matches!(first[0].1, Outcome::Queued { .. }),
        "{:?}",
        first[0].1
    );
    let (state, path, ticket) = standing(&project, &id);
    assert_eq!(state, "Some(Queued)");
    assert_eq!(path, None, "there is no file until it is collected");
    assert_eq!(ticket.as_deref(), Some("operations/shot"));
    assert!(
        project
            .asset(&id)
            .is_some_and(|asset| asset.queued_at.is_some()),
        "queued_at is what says whether a wait has gone on too long"
    );

    let second = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let Outcome::Generated { path, bytes, .. } = &second[0].1 else {
        panic!("expected a generation, got {:?}", second[0].1);
    };
    assert_eq!(*bytes, 9);
    assert!(path.as_str().starts_with("generated/shot-"));
    assert!(path.resolve(&dir).is_file(), "the video is not on disk");

    let (state, on_asset, ticket) = standing(&project, &id);
    assert_eq!(state, "Some(Generated)");
    assert_eq!(on_asset.as_deref(), Some(path.as_str()));
    assert_eq!(ticket, None, "a collected ticket is nobody's to poll again");
    assert_eq!(
        project.asset(&id).and_then(|a| a.estimated_cost_cents),
        Some(96),
        "eight seconds of Fast at 1080p, calculated — never billed"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The ticket is in the document, so a process that never comes back is not a
/// generation lost. This is the same project loaded fresh, which is what
/// opening the app does.
#[test]
fn a_generation_survives_the_process_that_started_it() {
    let (dir, mut project, id) = sketched("resume", "a city at dawn");
    let provider = Mock::answering(
        "shot",
        vec![Answer::Waiting, Answer::Ready(b"LATER".to_vec())],
    );

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    project.save(&dir).expect("save the project");

    // Everything the first run knew is gone but the document.
    let mut reopened = scorsese_core::Project::load(&dir).expect("reopen the project");
    let waiting = collect(&mut reopened, &dir, &provider).expect("a sweep");
    assert!(matches!(waiting[0].1, Outcome::Waiting { .. }));

    let collected = collect(&mut reopened, &dir, &provider).expect("a sweep");
    assert!(
        matches!(collected[0].1, Outcome::Generated { .. }),
        "{:?}",
        collected[0].1
    );
    assert_eq!(standing(&reopened, &id).0, "Some(Generated)");
    assert_eq!(provider.submissions(), 1, "resuming must never re-submit");
    std::fs::remove_dir_all(&dir).ok();
}

/// A sweep on opening a project must not be able to start anything. One that
/// could would eventually start twenty.
#[test]
fn collecting_never_submits() {
    let (dir, mut project, id) = sketched("sweep", "a harbour at night");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"X".to_vec())]);

    let swept = collect(&mut project, &dir, &provider).expect("a sweep");
    assert!(swept.is_empty(), "nothing was in flight: {swept:?}");
    assert_eq!(provider.submissions(), 0);
    assert_eq!(standing(&project, &id).0, "Some(Sketch)");
    std::fs::remove_dir_all(&dir).ok();
}

/// A refused brief goes back to being a sketch: it is a prompt to edit and try
/// again, and an asset still claiming to be queued would be polled for ever.
#[test]
fn a_refused_brief_goes_back_to_being_a_sketch() {
    let (dir, mut project, id) = sketched("refused", "something the model will not make");
    let provider = Mock::answering(
        "shot",
        vec![Answer::Failed(String::from("the prompt was rejected"))],
    );

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let second = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let Outcome::Failed { message } = &second[0].1 else {
        panic!("expected a refusal, got {:?}", second[0].1);
    };
    assert!(message.contains("rejected"), "{message}");

    let (state, path, ticket) = standing(&project, &id);
    assert_eq!(state, "Some(Sketch)");
    assert_eq!(path, None);
    assert_eq!(ticket, None, "a dead ticket is not worth polling");
    std::fs::remove_dir_all(&dir).ok();
}

/// The failure that actually happens at three in the morning: several shots,
/// one of them refused. The others must be finished and said to be.
#[test]
fn one_shot_failing_leaves_the_rest_generated() {
    let (dir, mut project) = crate::common::project("partial");
    for (id, prompt) in [("a", "a wide shot"), ("b", "a close-up"), ("c", "a pan")] {
        project.assets.push(scorsese_core::Asset::sketch(
            scorsese_core::AssetId::new(id),
            scorsese_core::AssetKind::GeneratedVideo,
            prompt,
        ));
    }
    let provider = Mock::answering("a", vec![Answer::Ready(b"AAA".to_vec())])
        .and("b", vec![Answer::Failed(String::from("no"))])
        .and("c", vec![Answer::Ready(b"CCC".to_vec())]);

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let second = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    let by_id = |want: &str| {
        second
            .iter()
            .find(|(id, _)| id.as_str() == want)
            .map(|(_, outcome)| outcome)
            .expect("every asset is reported on")
    };
    assert!(matches!(by_id("a"), Outcome::Generated { .. }));
    assert!(matches!(by_id("b"), Outcome::Failed { .. }));
    assert!(matches!(by_id("c"), Outcome::Generated { .. }));
    std::fs::remove_dir_all(&dir).ok();
}

/// The tally has to survive the outcome it was recorded on being replaced.
///
/// A shot submitted on this run is `Queued` — the outcome that knows it was
/// paid for. The wait then collects it and replaces that with `Generated`,
/// which cannot know whether the money went today or last week. A live run
/// reported spending $0.00 on a shot it had just paid for, which is exactly
/// the wrong direction for a number about money to be wrong in.
#[test]
fn a_run_that_waits_still_says_what_it_spent() {
    let (dir, mut project, _) = sketched("waited", "a kite on an empty beach");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"MP4".to_vec())]);

    let run = scorsese_providers::video::generate_waiting(
        &mut project,
        &dir,
        &provider,
        Budget::unlimited(0),
        std::time::Duration::from_secs(60),
        |_| {},
    )
    .expect("a run");

    assert!(
        matches!(run.outcomes[0].1, Outcome::Generated { .. }),
        "{:?}",
        run.outcomes[0].1
    );
    assert_eq!(run.spent_cents, 96, "the shot was paid for on this run");
    assert_eq!(run.in_flight(), 0);
    std::fs::remove_dir_all(&dir).ok();
}
