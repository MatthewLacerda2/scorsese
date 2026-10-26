//! Sorting a vendor refusal into advice somebody can act on.
//!
//! **Every body here is one the live API actually returned**, copied verbatim
//! from a hand-run probe rather than invented from documentation. That matters
//! for one case in particular: read from the docs, a 401 is one thing. In
//! practice it is two, and they have opposite fixes.

use scorsese_providers::api::elevenlabs::refusal::{Refusal, Wait};

/// What the API answered when the key lacked a scope.
const MISSING_SCOPE: &str = r#"{"detail":{"type":"authentication_error","code":"unauthorized",
    "message":"The API key you used is missing the permission voices_read to execute this operation.",
    "status":"missing_permissions","request_id":"8533cd8d"}}"#;

/// What it answered when a free account asked for a Voice Library voice.
const FREE_PLAN: &str = r#"{"detail":{"type":"payment_required","code":"paid_plan_required",
    "message":"Free users cannot use library voices via the API. Please upgrade your subscription to use this voice.",
    "status":"payment_required","request_id":"2d54ad65"}}"#;

/// A valid key that lacks a scope is **not** a bad key, and saying so sends
/// somebody to stare at the one thing that is not the problem. The fix is a
/// checkbox in the vendor's dashboard, on a key that already works.
#[test]
fn a_missing_scope_is_not_a_bad_key() {
    let refusal = Refusal::of(401, "any-voice", MISSING_SCOPE);

    assert!(
        matches!(refusal, Refusal::MissingPermission { .. }),
        "{refusal:?}"
    );
    let said = refusal.to_string();
    assert!(said.contains("voices_read"), "it names the scope: {said}");
    assert!(said.contains("dashboard"), "and where to fix it: {said}");
    assert!(
        !said.contains(".env"),
        "and never sends them to the key, which is fine: {said}"
    );
}

#[test]
fn a_401_without_that_status_is_a_bad_key() {
    let refusal = Refusal::of(
        401,
        "any-voice",
        r#"{"detail":{"message":"Invalid API key"}}"#,
    );

    assert!(matches!(refusal, Refusal::BadKey { .. }), "{refusal:?}");
    assert!(refusal.to_string().contains("ELEVENLABS_API_KEY"));
}

/// A free account is refused **per voice**, not per endpoint: the same key
/// speaks perfectly well with another voice. So this must not read as "upgrade
/// to use scorsese".
#[test]
fn a_voice_the_plan_does_not_cover_says_so() {
    let refusal = Refusal::of(402, "21m00Tcm4TlvDq8ikWAM", FREE_PLAN);

    assert!(
        matches!(refusal, Refusal::PlanRequired { .. }),
        "{refusal:?}"
    );
    assert!(
        refusal.to_string().contains("library voices"),
        "the vendor's own sentence survives: {refusal}"
    );
}

/// Expected rather than exceptional — every Default voice expires 2026-12-31 —
/// so it names the voice and says what to do instead.
#[test]
fn a_withdrawn_voice_names_itself() {
    let refusal = Refusal::of(404, "gone-voice", "{}");

    assert!(matches!(refusal, Refusal::VoiceGone { .. }), "{refusal:?}");
    let said = refusal.to_string();
    assert!(said.contains("gone-voice"), "{said}");
    assert!(said.contains("choose another"), "{said}");
}

/// Two causes behind one status, and the advice differs: our own requests
/// stacked up clear in a moment, and a loaded service does not.
#[test]
fn the_two_kinds_of_busy_advise_differently() {
    let ours = Refusal::of(
        429,
        "v",
        r#"{"detail":{"code":"too_many_concurrent_requests","message":"slow down"}}"#,
    );
    let theirs = Refusal::of(
        429,
        "v",
        r#"{"detail":{"code":"system_busy","message":"busy"}}"#,
    );

    assert!(
        matches!(
            ours,
            Refusal::Busy {
                wait: Wait::Moment,
                ..
            }
        ),
        "{ours:?}"
    );
    assert!(
        matches!(
            theirs,
            Refusal::Busy {
                wait: Wait::While,
                ..
            }
        ),
        "{theirs:?}"
    );
    assert_ne!(ours.to_string(), theirs.to_string());
}

/// The code is consulted before the status, because an exhausted quota has been
/// seen behind more than one status and its code says so either way.
#[test]
fn an_exhausted_quota_is_recognised_whatever_the_status() {
    for status in [401, 402, 429] {
        let refusal = Refusal::of(
            status,
            "v",
            r#"{"detail":{"code":"quota_exceeded","message":"no credits"}}"#,
        );
        assert!(
            matches!(refusal, Refusal::OutOfCredits { .. }),
            "{status}: {refusal:?}"
        );
    }
}

/// Only [`Refusal::Busy`] is worth trying again unchanged. A missing scope and
/// a spent quota are both fixable and neither is fixed by scorsese retrying.
#[test]
fn only_a_busy_vendor_is_worth_retrying() {
    let busy = Refusal::of(429, "v", r#"{"detail":{"code":"system_busy"}}"#);
    assert!(busy.is_worth_retrying());

    for refusal in [
        Refusal::of(401, "v", MISSING_SCOPE),
        Refusal::of(402, "v", FREE_PLAN),
        Refusal::of(404, "v", "{}"),
    ] {
        assert!(!refusal.is_worth_retrying(), "{refusal:?}");
    }
}

/// A body that is not the vendor's envelope — its web framework's own
/// validation errors put a list in `detail` — must not become "unreadable
/// reply". The transport kept the body whole, so it is carried through.
#[test]
fn a_body_that_is_not_the_envelope_is_still_carried_through() {
    let refusal = Refusal::of(422, "v", r#"{"detail":[{"msg":"text too long"}]}"#);

    assert!(matches!(refusal, Refusal::Invalid { .. }), "{refusal:?}");
    assert!(
        refusal.to_string().contains("text too long"),
        "the message survives: {refusal}"
    );
}
