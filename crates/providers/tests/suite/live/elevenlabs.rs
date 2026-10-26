//! What ElevenLabs' answers mean. The listing and the refusals are bodies
//! the live API really sent; the design reply is hand-written until a live
//! run records one.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use scorsese_providers::api::elevenlabs::design::{DesignReply, PASSAGE as PASSAGES, PROMPT};
use scorsese_providers::api::elevenlabs::speech::Timed;
use scorsese_providers::api::elevenlabs::voices::Listing;
use scorsese_providers::live::Verdict;
use scorsese_providers::live::elevenlabs::{
    DESCRIPTION, PASSAGE, cost, design_step, listing_step, speech_step,
};

use super::{parsed, refused};

/// Captured: a key missing the `voices_read` scope.
const MISSING_SCOPE: &str = r#"{"detail":{"type":"authentication_error","code":"unauthorized",
    "message":"The API key you used is missing the permission voices_read to execute this operation.",
    "status":"missing_permissions","request_id":"8533cd8d"}}"#;

/// Captured: a free account asking for a library voice.
const FREE_PLAN: &str = r#"{"detail":{"type":"payment_required","code":"paid_plan_required",
    "message":"Free users cannot use library voices via the API. Please upgrade your subscription to use this voice.",
    "status":"payment_required","request_id":"2d54ad65"}}"#;

#[test]
fn the_captured_premade_listing_is_ok_and_supplies_a_voice() {
    let (step, voice) = listing_step(Ok(parsed("elevenlabs/premade.json")));
    assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
    assert!(!voice.unwrap().is_empty());
}

#[test]
fn an_empty_listing_is_a_shape_change_and_leaves_nothing_to_speak_with() {
    let (step, voice) = listing_step(Ok(Listing::default()));
    assert!(matches!(step.verdict, Verdict::ShapeChanged { .. }));
    assert_eq!(voice, None);
}

/// The key is fine and lacks a scope: still an auth failure, but one that
/// says so in the vendor's words rather than blaming the key.
#[test]
fn a_missing_scope_is_an_auth_failure_that_names_the_scope() {
    let (step, _) = listing_step(Err(refused(401, MISSING_SCOPE)));
    let Verdict::AuthFailed { said } = &step.verdict else {
        panic!("{step:?}")
    };
    assert!(
        said.contains("voices_read") && said.contains("dashboard"),
        "{said}"
    );
}

#[test]
fn a_plan_that_does_not_cover_the_call_is_a_refusal_not_an_auth_failure() {
    let step = speech_step(Err(refused(402, FREE_PLAN)));
    assert!(matches!(&step.verdict, Verdict::Refused { said } if said.contains("library voices")));
}

/// A reply as the timestamps endpoint sends it: `audio` in base64, with
/// one character timed.
fn timed(audio: &[u8], timed: bool) -> Timed {
    let alignment = r#","alignment":{"characters":["a"],
        "character_start_times_seconds":[0.0],"character_end_times_seconds":[0.1]}"#;
    let json = format!(
        r#"{{"audio_base64":"{}"{}}}"#,
        STANDARD.encode(audio),
        if timed { alignment } else { "" }
    );
    serde_json::from_str(&json).expect("a timed reply")
}

#[test]
fn speech_must_come_back_as_a_timed_mp3() {
    assert_eq!(
        speech_step(Ok(timed(b"ID3\x04rest", true))).verdict,
        Verdict::Ok
    );
    assert_eq!(
        speech_step(Ok(timed(&[0xFF, 0xF3, 0x44], true))).verdict,
        Verdict::Ok
    );
    let json = speech_step(Ok(timed(br#"{"detail":"x"}"#, true)));
    assert!(matches!(&json.verdict, Verdict::ShapeChanged { field } if field.contains("JSON")));
    let untimed = speech_step(Ok(timed(b"ID3\x04rest", false)));
    assert!(
        matches!(&untimed.verdict, Verdict::ShapeChanged { field } if field.contains("alignment"))
    );
}

#[test]
fn a_design_is_three_candidates_each_with_an_id_and_an_mp3() {
    let reply: DesignReply = parsed("elevenlabs/design.json");
    assert_eq!(design_step(Ok(reply.clone())).verdict, Verdict::Ok);

    let mut short = reply.clone();
    short.previews.pop();
    assert!(matches!(
        design_step(Ok(short)).verdict,
        Verdict::ShapeChanged { .. }
    ));

    let mut silent = reply;
    silent.previews[1].audio_base_64 = String::from("not base64!");
    let step = design_step(Ok(silent));
    assert!(
        matches!(&step.verdict, Verdict::ShapeChanged { field } if field.contains("audio_base_64"))
    );
}

/// What the check sends is inside the vendor's limits, and cheap: a cent to
/// speak a word, a cent for the shortest passage a design accepts, a cent for
/// the shortest track.
#[test]
fn the_calls_stay_inside_the_vendors_limits_and_cost_three_cents() {
    assert_eq!(
        PASSAGE.chars().count(),
        *PASSAGES.start(),
        "the minimum is the price"
    );
    assert!(PROMPT.contains(&DESCRIPTION.chars().count()));
    assert_eq!(cost(), 3);
}

/// The music probe (#1418): a three-second instrumental track.
mod music {
    use std::time::Duration;

    use scorsese_providers::api::elevenlabs::music::MODEL_ID;
    use scorsese_providers::live::Verdict;
    use scorsese_providers::live::elevenlabs::{TRACK_SECONDS, music_cents, music_step, track};

    use super::{FREE_PLAN, MISSING_SCOPE, refused};

    #[test]
    fn an_mp3_is_ok_and_says_how_long_it_took() {
        let step = music_step(Ok(b"ID3\x04rest".to_vec()), Duration::from_millis(4_200));
        assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
        assert_eq!(step.notes, ["8 bytes of MP3, back in 4.2 s"]);
    }

    #[test]
    fn json_where_the_track_should_be_is_a_shape_change() {
        let step = music_step(Ok(br#"{"detail":"x"}"#.to_vec()), Duration::ZERO);
        assert!(
            matches!(&step.verdict, Verdict::ShapeChanged { field } if field.contains("JSON")),
            "{step:?}"
        );
    }

    /// A refusal is sorted by the one reader of the vendor's refusals.
    #[test]
    fn a_refusal_is_sorted_as_the_vendor_meant_it() {
        let scope = music_step(Err(refused(401, MISSING_SCOPE)), Duration::ZERO);
        assert!(
            matches!(scope.verdict, Verdict::AuthFailed { .. }),
            "{scope:?}"
        );
        let plan = music_step(Err(refused(402, FREE_PLAN)), Duration::ZERO);
        assert!(matches!(plan.verdict, Verdict::Refused { .. }), "{plan:?}");
    }

    /// The shortest track the vendor composes, on the product's model, with
    /// no singing: a cent, rounded up from three-quarters of one.
    #[test]
    fn the_track_is_the_shortest_instrumental_and_costs_a_cent() {
        let body = track();
        assert_eq!(body.music_length_ms, 3_000);
        assert_eq!(body.music_length_ms, TRACK_SECONDS * 1_000);
        assert_eq!(body.model_id, MODEL_ID);
        assert!(body.force_instrumental);
        assert_eq!(music_cents(), 1);
    }
}
