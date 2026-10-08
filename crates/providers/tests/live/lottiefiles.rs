//! LottieFiles' part (#910): the free, keyless search, judged against the
//! body LottieFiles sent on 2026-10-08 — never the endpoint itself.

use scorsese_providers::api::lottiefiles::{Connection, Reply, Searched};
use scorsese_providers::credentials::{Environment, Settings};
use scorsese_providers::live::lottiefiles::search_step;
use scorsese_providers::live::{Options, Vendor, Verdict, plan, total};

use super::{parsed, refused};

/// The captured search, as the client reads it.
fn found() -> Connection {
    let reply: Reply<Searched> = parsed("lottiefiles/search.json");
    reply.into_data().unwrap().found
}

fn changed(found: Connection) -> String {
    match search_step(Ok(found)).verdict {
        Verdict::ShapeChanged { field } => field,
        other => panic!("expected a shape change, got {other:?}"),
    }
}

#[test]
fn the_captured_search_is_the_shape_the_library_reads() {
    let step = search_step(Ok(found()));
    assert_eq!(step.verdict, Verdict::Ok);
    assert_eq!(step.notes, ["3 results of 83127"]);
}

#[test]
fn a_result_without_its_json_is_named() {
    let mut found = found();
    found.edges[1].node.json_url = None;
    assert!(changed(found).contains("jsonUrl: result 121035"));
}

/// One old upload without `metadata` is not the API changing; every result
/// without a field is.
#[test]
fn a_measurement_is_missing_only_when_no_result_carries_it() {
    let mut found = found();
    found.edges[0].node.metadata = None;
    assert_eq!(search_step(Ok(found.clone())).verdict, Verdict::Ok);
    for edge in &mut found.edges {
        if let Some(metadata) = &mut edge.node.metadata {
            metadata.frame_rate = None;
        }
    }
    assert!(changed(found).contains("metadata.frameRate"));
}

#[test]
fn an_empty_search_is_a_shape_change() {
    let mut found = found();
    found.edges.clear();
    assert!(changed(found).starts_with("edges"));
}

/// The day anonymous access goes, the verdict says what the fallback is —
/// whether it arrives as a status or as a GraphQL error answered `200`.
#[test]
fn a_withdrawn_anonymous_access_names_the_fallback() {
    for error in [
        refused(401, "{}"),
        refused(200, "Unauthorized: a token is required"),
    ] {
        let Verdict::AuthFailed { said } = search_step(Err(error)).verdict else {
            panic!("a withdrawal is auth failed");
        };
        assert!(said.contains("maintainer's LottieFiles account"), "{said}");
    }
    let other = search_step(Err(refused(200, "String cannot represent 5")));
    assert!(matches!(other.verdict, Verdict::Refused { .. }));
}

/// Asked for, it is planned with no key and costs nothing; by default it is
/// skipped, so a run built from defaults reaches no vendor at all.
#[test]
fn the_keyless_vendor_is_free_and_off_unless_asked_for() {
    let asked = Options {
        keyless: true,
        ..Options::default()
    };
    let lottie = |options: &Options| {
        plan(options, &Environment::default(), &Settings::default())
            .into_iter()
            .find(|p| p.vendor == Vendor::LottieFiles)
            .unwrap()
    };
    let planned = lottie(&asked);
    assert!(planned.skipped.is_none());
    assert!(planned.calls[0].contains("free, no key"));
    assert_eq!(total(&[planned]), 0);
    assert!(lottie(&Options::default()).skipped.is_some());
}
