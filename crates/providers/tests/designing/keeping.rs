//! Keeping a candidate — and the record that outlives the voice.
//!
//! The tests here are about the one thing in scorsese with an effect outside
//! the project directory. What is asserted is not that a voice was created —
//! the fake will always say so — but that **what made it** was written down, in
//! the project, where the voice itself cannot go.

use scorsese_providers::credentials::Budget;
use scorsese_providers::voices::design::{design, designed, keep, spent};

use super::fake::Fake;
use super::{brief, root};

/// The whole point of the ledger: the id travels with the project and the voice
/// does not, so the description and the seed have to travel instead.
#[test]
fn what_made_the_voice_is_written_into_the_project() {
    let dir = root("design-ledger");
    let studio = Fake::offering(&["a", "b", "c"]);
    let brief = brief("a Brazilian woman in her forties", Some(42));
    design(&dir, &studio, &brief, Budget::unlimited(0)).expect("the design");

    let kept = keep(&dir, &studio, "b", "Narrator").expect("keeping the second candidate");

    let ledger = designed(&dir);
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].voice_id, kept.voice.id);
    assert_eq!(ledger[0].seed, Some(42), "the seed is half the recovery");
    assert_eq!(ledger[0].prompt, brief.prompt, "the other half");
    assert_eq!(ledger[0].passage, brief.passage);
    assert!(ledger[0].estimated_cost_cents > 0);
}

/// The two that were heard and passed over go back to the vendor, which is free
/// and is what the endpoint asks for. Forgetting is the only way to get it
/// wrong, so it is asserted.
#[test]
fn the_candidates_that_lost_are_reported_as_such() {
    let dir = root("design-passed-over");
    let studio = Fake::offering(&["a", "b", "c"]);
    design(
        &dir,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect("the design");

    keep(&dir, &studio, "b", "Narrator").expect("keeping one");

    let kept = studio.kept.borrow();
    assert_eq!(kept[0].chosen, "b");
    assert_eq!(kept[0].name, "Narrator");
    assert_eq!(
        kept[0].passed_over,
        vec![String::from("a"), String::from("c")]
    );
}

/// One design is one charge, however many voices come out of it. A total that
/// counted the design again per voice would report a spend nobody made.
#[test]
fn keeping_two_candidates_from_one_design_is_one_charge() {
    let dir = root("design-two-kept");
    let studio = Fake::offering(&["a", "b", "c"]);
    let designing = design(
        &dir,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect("the design");

    keep(&dir, &studio, "a", "First").expect("keeping one");
    keep(&dir, &studio, "c", "Second").expect("keeping another");

    assert_eq!(designed(&dir).len(), 2, "two voices exist");
    assert_eq!(
        spent(&dir),
        designing.estimate.cents,
        "one design was billed once"
    );
}

/// A candidate id from nowhere is a refusal that says nothing was created —
/// which is the sentence that stops somebody hunting for a voice that does not
/// exist.
#[test]
fn a_candidate_no_design_offered_is_refused() {
    let dir = root("design-unknown");
    let studio = Fake::offering(&["a", "b", "c"]);
    design(
        &dir,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect("the design");

    let error = keep(&dir, &studio, "not-a-candidate", "Narrator")
        .expect_err("an unknown candidate cannot be kept");
    assert!(error.to_string().contains("not-a-candidate"), "{error}");
    assert!(
        studio.kept.borrow().is_empty(),
        "the vendor was asked anyway"
    );
}

/// Designing in one project and keeping in another must not work by accident:
/// the record that makes a voice recoverable lives in the project that paid for
/// it.
#[test]
fn a_candidate_from_another_project_is_not_found() {
    let studio = Fake::offering(&["a", "b", "c"]);
    let designed_in = root("design-here");
    design(
        &designed_in,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect("the design");

    let error = keep(&root("design-elsewhere"), &studio, "a", "Narrator")
        .expect_err("another project never offered that candidate");
    assert!(
        error.to_string().contains("no design in this project"),
        "{error}"
    );
}
