//! What a design is quoted at before anybody agrees to it.

use scorsese_providers::credentials::Budget;
use scorsese_providers::quote::Spend;
use scorsese_providers::voices::design::{design, estimate, quote};

use super::fake::Fake;
use super::{brief, passage, root};

/// Charged once, at the estimate — never three times for three candidates.
#[test]
fn a_new_design_is_charged_once_at_the_estimate() {
    let dir = root("design-quote");
    let quoted = quote(&dir, &brief("a calm narrator", Some(3))).expect("quote");

    assert_eq!(quoted.spend, Spend::VoiceDesign);
    assert_eq!(quoted.items.len(), 1);
    assert_eq!(
        quoted.cents(),
        estimate(&passage()).expect("estimate").cents
    );
}

/// Samples already on disk are the answer to *has this been paid for*, so the
/// quote is free and a surface asks nobody anything.
#[test]
fn a_design_already_paid_for_is_free() {
    let dir = root("design-quote-paid");
    let brief = brief("a calm narrator", Some(4));
    design(
        &dir,
        &Fake::offering(&["a", "b", "c"]),
        &brief,
        Budget::unlimited(0),
    )
    .expect("the design");

    assert!(quote(&dir, &brief).expect("quote").is_free());
}

/// A different seed is a different design, so a token for one cannot pay for
/// the other.
#[test]
fn a_different_brief_is_a_different_digest() {
    let dir = root("design-quote-seed");
    let one = quote(&dir, &brief("a calm narrator", Some(5))).expect("quote");
    let two = quote(&dir, &brief("a calm narrator", Some(6))).expect("quote");
    assert_ne!(one.digest(), two.digest());
}
