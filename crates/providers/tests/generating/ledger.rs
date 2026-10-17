//! What a project has spent counts every take it bought, not only the ones its
//! assets still show (#1059).
//!
//! An asset's figure is what its current file cost, so a shot bought again
//! overwrote the first take's price and the ceiling forgot it. The ledger beside
//! `project.json` is written as each brief is sent, and is what is counted.

use scorsese_core::ProjectPath;
use scorsese_providers::credentials::Budget;
use scorsese_providers::spending::{GENERATIONS_FILE, history, so_far};
use scorsese_providers::video::generate;

use crate::mock::{Answer, Mock};
use crate::sketched;

#[test]
fn a_shot_bought_twice_is_counted_twice_and_named_a_superseded_take() {
    let (dir, mut project, id) = sketched("twice", "a shop sign in pseudo-Chinese");
    let provider = Mock::answering(
        "shot",
        vec![
            Answer::Ready(b"FIRST".to_vec()),
            Answer::Ready(b"SECOND".to_vec()),
        ],
    );
    let run = |project: &mut _| generate(project, &dir, &provider, Budget::unlimited(0));
    run(&mut project).expect("submitted");
    run(&mut project).expect("collected");
    let one = project
        .asset(&id)
        .and_then(|asset| asset.estimated_cost_cents);
    let one = one.expect("the first take is priced");
    project.asset_mut(&id).expect("the asset").prompt = Some(String::from("a sign in English"));
    run(&mut project).expect("submitted again");
    run(&mut project).expect("collected again");

    let spent = so_far(&project, &dir);
    assert_eq!(spent.generation_cents, 2 * one, "both takes were paid for");
    assert_eq!(spent.total(), 2 * one, "and the ceiling counts both");
    assert_eq!(spent.superseded.takes, 1);
    assert_eq!(spent.superseded.cents, one);
    assert_eq!(
        project
            .asset(&id)
            .and_then(|asset| asset.estimated_cost_cents),
        Some(one),
        "the asset still says what its current file cost"
    );
    let says = spent.says();
    assert!(says.contains("\n1 superseded take, $"), "{says}");

    project.assets.retain(|asset| asset.id != id);
    let removed = so_far(&project, &dir);
    assert_eq!(
        removed.total(),
        2 * one,
        "removing the shot refunds nothing"
    );
    assert_eq!(
        (removed.superseded.takes, removed.superseded.cents),
        (2, 2 * one)
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_shot_in_flight_is_spent_from_the_moment_it_is_sent() {
    let (dir, mut project, _) = sketched("flight", "a crane shot over a market");
    let provider = Mock::answering("shot", vec![Answer::Waiting]);

    let done = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    let spent = so_far(&project, &dir);
    assert!(spent.generation_cents > 0);
    assert_eq!(spent.generation_cents, done[0].1.spent_cents());
    assert_eq!(
        spent.superseded.takes, 0,
        "a take in flight is not superseded"
    );
    assert!(!spent.says().contains("superseded"), "{}", spent.says());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_project_older_than_the_ledger_begins_it_with_its_current_takes() {
    let (dir, mut project, id) = sketched("older", "a neon alley in the rain");
    let asset = project.asset_mut(&id).expect("the asset");
    asset.path = Some(ProjectPath::new("generated/shot-0123abcd.mp4"));
    asset.estimated_cost_cents = Some(40);

    let before = so_far(&project, &dir);
    assert_eq!((before.generation_cents, before.superseded.takes), (40, 0));
    assert!(
        !dir.join(GENERATIONS_FILE).exists(),
        "reading writes nothing"
    );

    let provider = Mock::answering("shot", vec![Answer::Ready(b"NEW".to_vec())]);
    let done = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    let lines = history(&project, &dir);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines[0].seeded && lines[0].brief == "0123abcd",
        "{:?}",
        lines[0]
    );
    assert!(lines[0].sent_at.is_none() && lines[1].sent_at.is_some());
    let spent = so_far(&project, &dir);
    assert_eq!(spent.generation_cents, 40 + done[0].1.spent_cents());
    assert_eq!(
        spent.superseded.takes, 0,
        "the old take is still on the asset"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_ledger_that_does_not_read_is_never_written_over() {
    let (dir, mut project, _) = sketched("damaged", "a rooftop at dawn");
    std::fs::write(dir.join(GENERATIONS_FILE), "{ not a ledger").expect("write");
    let provider = Mock::answering("shot", vec![Answer::Waiting]);

    let stopped = generate(&mut project, &dir, &provider, Budget::unlimited(0));

    assert!(
        stopped.is_err(),
        "the run stops rather than lose the history"
    );
    let text = std::fs::read_to_string(dir.join(GENERATIONS_FILE)).expect("read");
    assert_eq!(text, "{ not a ledger");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_ticket_lost_after_it_was_sent_is_a_superseded_take() {
    let (dir, mut project, _) = sketched("lost", "a train through the desert");
    let answers = vec![Answer::Lost, Answer::Waiting, Answer::Ready(b"V".to_vec())];
    let provider = Mock::answering("shot", answers);
    let run = |project: &mut _| generate(project, &dir, &provider, Budget::unlimited(0));
    let sent = run(&mut project).expect("submitted")[0].1.spent_cents();
    let superseded = |project: &_| {
        let spent = so_far(project, &dir);
        (
            spent.total(),
            spent.superseded.takes,
            spent.superseded.cents,
        )
    };

    run(&mut project).expect("the ticket is lost");
    assert_eq!(superseded(&project), (sent, 1, sent), "sent, and gone");
    run(&mut project).expect("the same brief, submitted again");
    run(&mut project).expect("waiting");
    assert_eq!(
        superseded(&project),
        (2 * sent, 1, sent),
        "the new one is in flight"
    );
    run(&mut project).expect("collected");
    assert_eq!(
        superseded(&project),
        (2 * sent, 1, sent),
        "one take shown, one lost"
    );
    std::fs::remove_dir_all(&dir).ok();
}
