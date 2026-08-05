//! What happens when a voice will not do — the half of this issue with a date
//! on it.

use scorsese_providers::voices::{
    Availability, Filters, MAX_PAGE_SIZE, Unusable, VoiceError, library, require,
};

use super::fake::Fake;
use super::{root, voice};

/// The case that fires on 2027-01-01: an id that was fine yesterday.
#[test]
fn a_withdrawn_voice_is_a_refusal_that_says_what_to_do() {
    let fake = Fake::answering(Availability::Unusable(Unusable::Gone));

    let error = require(&fake, "21m00Tcm4TlvDq8ikWAM").expect_err("a gone voice cannot be used");
    let said = error.to_string();
    assert!(said.contains("21m00Tcm4TlvDq8ikWAM"), "{said}");
    assert!(said.contains("2026-12-31"), "{said}");
    assert!(
        said.contains("nothing has been changed"),
        "the asset is still editable, and the message has to say so: {said}"
    );
}

/// A Voice Library voice a free account may not generate with exists perfectly
/// well. Reporting it as deleted would send somebody hunting for a replacement
/// they do not need.
#[test]
fn a_voice_the_plan_forbids_is_not_reported_as_deleted() {
    let fake = Fake::answering(Availability::Unusable(Unusable::NotOnThisPlan {
        said: String::from("Free users cannot use library voices via the API."),
    }));

    let said = require(&fake, "somebody-elses-voice")
        .expect_err("a forbidden voice cannot be used")
        .to_string();
    assert!(
        said.contains("Free users cannot use library voices"),
        "{said}"
    );
    assert!(
        !said.contains("2026-12-31"),
        "this is a plan limit, not an expiry: {said}"
    );
}

/// **The one that must never be confused.** A timeout says nothing whatever
/// about the voice, and reporting it as gone would let a flaky connection
/// delete somebody's choice of narrator.
#[test]
fn an_unreachable_provider_never_reads_as_a_missing_voice() {
    let mut fake = Fake::listing(Vec::new());
    fake.unreachable = true;

    match require(&fake, "v1").expect_err("an unreachable provider cannot answer") {
        VoiceError::Provider(_) => {}
        other => panic!("a network failure must stay a network failure, not {other:?}"),
    }
}

/// A voice that is there is simply handed back — the case that has to keep
/// working for any of the above to be worth anything.
#[test]
fn a_voice_that_is_there_comes_back_whole() {
    let fake = Fake::answering(Availability::Available(voice("v1", "Ana")));
    let found = require(&fake, "v1").expect("an available voice");
    assert_eq!(found, voice("v1", "Ana"));
}

/// The free-plan refusal on the listing itself, which is the other half of the
/// same limit — and it has to name the built-in voices as the way out.
#[test]
fn the_voice_library_refusal_points_at_the_built_in_voices() {
    let dir = root("voices-free-plan");
    let mut fake = Fake::listing(Vec::new());
    fake.free_plan = true;

    let said = library(&dir, &fake, &Filters::default(), false)
        .expect_err("a free plan cannot read the library")
        .to_string();
    assert!(said.contains("free plan"), "{said}");
    assert!(said.contains("scorsese voices"), "{said}");
}

/// Refused before a round trip, because being told a number is too big is worth
/// no seconds at all.
#[test]
fn too_large_a_page_is_refused_without_asking() {
    let dir = root("voices-page-size");
    let fake = Fake::listing(Vec::new());
    let filters = Filters {
        page_size: Some(MAX_PAGE_SIZE + 1),
        ..Filters::default()
    };

    let error = library(&dir, &fake, &filters, false).expect_err("101 is too many");
    assert!(matches!(error, VoiceError::PageTooLarge { asked: 101 }));
    assert_eq!(fake.calls(), 0, "the provider should never have been asked");
}
