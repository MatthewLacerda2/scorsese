//! sketch → queued → generated: the states a brief passes through, and what
//! the asset says once it has.
//!
//! A brief the provider *refuses* is [`refusals`](super::refusals).

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

/// The gap this crate cannot close, and #249 is what closing it dishonestly
/// cost. A brief knows the length and the raster it asked for and knows nothing
/// about the sound Veo puts under a shot — and a `media` block written from the
/// brief is one no probe will ever replace, because a probe skips an asset that
/// already has one. So the whole block is left for whoever measures the file.
#[test]
fn a_generated_shot_arrives_unmeasured() {
    let (dir, mut project, id) = sketched("unmeasured", "a pit lane in 1976");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"MP4 BYTES".to_vec())]);

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a submit");
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a collection");

    assert!(
        project.asset(&id).expect("the asset").media.is_none(),
        "nothing here measured this file; probing is what fills this in"
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
