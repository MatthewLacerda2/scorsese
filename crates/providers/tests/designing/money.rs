//! The half of this feature that spends, and every way it must not.

use scorsese_providers::credentials::Budget;
use scorsese_providers::voices::design::{Brief, design, spent};

use super::fake::Fake;
use super::{brief, passage, root};

/// The guarantee the whole content-addressed store exists for: asking for the
/// same design twice is billed once. The likeliest way to ask twice is a
/// dropped connection, not a mistake.
#[test]
fn the_same_design_is_never_paid_for_twice() {
    let dir = root("design-cache");
    let studio = Fake::offering(&["a", "b", "c"]);
    let brief = brief("an older Brazilian man", Some(7));

    let first = design(&dir, &studio, &brief, Budget::unlimited(0)).expect("the first design");
    assert!(!first.already_paid);
    assert_eq!(first.spent_cents(), first.estimate.cents);

    let again = design(&dir, &studio, &brief, Budget::unlimited(0)).expect("the second design");
    assert_eq!(studio.designs(), 1, "the second design was billed again");
    assert!(again.already_paid);
    assert_eq!(again.spent_cents(), 0);
    assert_eq!(again.session.candidates, first.session.candidates);
}

/// A brief that differs anywhere is a different design, and has to be, or
/// editing the description would be answered with yesterday's voices.
#[test]
fn changing_the_seed_is_a_different_design() {
    let dir = root("design-seed");
    let studio = Fake::offering(&["a", "b", "c"]);

    design(
        &dir,
        &studio,
        &brief("a young narrator", Some(1)),
        Budget::unlimited(0),
    )
    .expect("the first design");
    design(
        &dir,
        &studio,
        &brief("a young narrator", Some(2)),
        Budget::unlimited(0),
    )
    .expect("the second design");

    assert_eq!(studio.designs(), 2);
}

/// The ceiling refuses **before** the vendor is asked, which is the only order
/// that saves any money.
#[test]
fn a_design_over_the_ceiling_is_never_sent() {
    let dir = root("design-ceiling");
    let studio = Fake::offering(&["a", "b", "c"]);

    let error = design(&dir, &studio, &brief("a narrator", None), Budget::new(1, 0))
        .expect_err("a design over the ceiling must be refused");

    assert_eq!(studio.designs(), 0, "the vendor was asked anyway");
    assert!(error.to_string().contains("ceiling"), "{error}");
}

/// The samples land where paid-for output lands, and the record beside them
/// carries the ids — an MP3 cannot, and without them the samples are three
/// sounds nobody can act on.
#[test]
fn the_samples_and_their_ids_are_written_into_generated() {
    let dir = root("design-files");
    let studio = Fake::offering(&["one", "two", "three"]);

    let designing = design(
        &dir,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect("the design");

    assert_eq!(designing.session.candidates.len(), 3);
    for sample in &designing.session.candidates {
        let path = sample.path.as_str();
        assert!(path.starts_with("generated/"), "{path}");
        assert!(
            sample.path.resolve(&dir).is_file(),
            "{path} was not written"
        );
    }
    let ids: Vec<&str> = designing
        .session
        .candidates
        .iter()
        .map(|sample| sample.generated_voice_id.as_str())
        .collect();
    assert_eq!(ids, ["one", "two", "three"], "the order came back changed");
}

/// Nothing has been designed, so nothing has been spent. The figure exists
/// before the feature is used, because a ceiling asks for it either way.
#[test]
fn a_project_that_has_designed_nothing_has_spent_nothing() {
    assert_eq!(spent(&root("design-unspent")), 0);
}

/// The lengths are the vendor's, and they are refused here rather than there:
/// a round trip to be told a number is too small says less than this does.
#[test]
fn a_description_or_a_passage_of_the_wrong_length_is_refused_locally() {
    let short = Brief::new("too short", &passage(), None, None)
        .expect_err("a nine-character description is refused");
    assert!(short.to_string().contains("20"), "{short}");

    let thin = Brief::new(
        "a perfectly reasonable description of a voice",
        "hello",
        None,
        None,
    )
    .expect_err("a five-character passage is refused");
    assert!(thin.to_string().contains("100"), "{thin}");
}

/// A plan that does not include Voice Design is a fact about somebody's
/// account, and the advice has to point at the voices that still work rather
/// than at their request.
#[test]
fn a_plan_refusal_points_at_the_voices_that_still_work() {
    let dir = root("design-plan");
    let mut studio = Fake::offering(&[]);
    studio.free_plan = true;

    let error = design(
        &dir,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect_err("a free plan cannot design");
    let said = error.to_string();
    assert!(said.contains("free plan"), "{said}");
    assert!(said.contains("scorsese voices"), "{said}");
}
