//! Forgetting a kept voice (#1381): deleted at the vendor, marked in the
//! ledger, and nothing that was paid for lost.

use scorsese_providers::credentials::Budget;
use scorsese_providers::voices::design::{design, designed, forget, keep, spent};

use super::fake::Fake;
use super::{brief, root};

/// A project with one design and its second candidate kept.
fn kept(label: &str, studio: &Fake) -> (std::path::PathBuf, String) {
    let dir = root(label);
    design(
        &dir,
        studio,
        &brief("a narrator", Some(7)),
        Budget::unlimited(0),
    )
    .expect("the design");
    let voice = keep(&dir, studio, "b", "Narrator")
        .expect("keeping one")
        .voice;
    (dir, voice.id)
}

/// The voice goes from the account; its entry stays, marked, still saying
/// what made it and what it cost.
#[test]
fn a_forgotten_voice_is_deleted_and_its_record_kept() {
    let studio = Fake::offering(&["a", "b", "c"]);
    let (dir, voice) = kept("forget-kept", &studio);
    let cost = spent(&dir);

    let forgotten = forget(&dir, &studio, &voice).expect("forgetting it");

    assert_eq!(*studio.forgotten.borrow(), vec![voice.clone()]);
    assert!(forgotten.forgotten);
    let ledger = designed(&dir);
    assert_eq!(ledger.len(), 1, "the entry is marked, never removed");
    assert!(!ledger[0].kept());
    assert_eq!(ledger[0].seed, Some(7));
    assert_eq!(spent(&dir), cost, "what the design cost is still counted");
    assert!(
        ledger[0].says().contains("forgotten"),
        "{}",
        ledger[0].says()
    );
}

/// A destructive call never guesses: an id the ledger does not keep reaches
/// no vendor, and neither does forgetting the same voice twice.
#[test]
fn only_a_voice_this_project_keeps_can_be_forgotten() {
    let studio = Fake::offering(&["a", "b", "c"]);
    let (dir, voice) = kept("forget-unknown", &studio);

    let error = forget(&dir, &studio, "someone-elses").expect_err("not ours");
    assert!(error.to_string().contains("someone-elses"), "{error}");
    assert!(error.to_string().contains("nothing was deleted"), "{error}");

    forget(&dir, &studio, &voice).expect("forgetting it");
    forget(&dir, &studio, &voice).expect_err("it is no longer kept");
    assert_eq!(studio.forgotten.borrow().len(), 1);
}

/// The vendor failing leaves the voice kept, so asking again can finish it.
#[test]
fn a_vendor_failure_forgets_nothing() {
    let mut studio = Fake::offering(&["a", "b", "c"]);
    let (dir, voice) = kept("forget-unreachable", &studio);
    studio.unreachable = true;

    forget(&dir, &studio, &voice).expect_err("the vendor did not answer");
    assert!(designed(&dir)[0].kept());
}

/// The design stays, so the same candidate can be kept again.
#[test]
fn a_forgotten_voice_can_be_kept_again() {
    let studio = Fake::offering(&["a", "b", "c"]);
    let (dir, voice) = kept("forget-again", &studio);
    forget(&dir, &studio, &voice).expect("forgetting it");

    keep(&dir, &studio, "b", "Narrator").expect("keeping it again");
    let ledger = designed(&dir);
    assert_eq!(ledger.len(), 2);
    assert!(ledger[1].kept());
}
