//! The sound probe (#1265): a half-second sound, played once.

use std::time::Duration;

use scorsese_core::MIN_SOUND_SECONDS;
use scorsese_providers::api::elevenlabs::sound::MODEL_ID;
use scorsese_providers::live::Verdict;
use scorsese_providers::live::elevenlabs::{SOUND_MILLIS, effect, sound_cents, sound_step};

#[test]
fn an_mp3_is_ok_and_says_how_long_it_took() {
    let step = sound_step(Ok(b"ID3\x04rest".to_vec()), Duration::from_millis(1_500));
    assert_eq!(step.call, "POST sound-generation");
    assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
    assert_eq!(step.notes, ["8 bytes of MP3, back in 1.5 s"]);
}

#[test]
fn json_where_the_sound_should_be_is_a_shape_change() {
    let step = sound_step(Ok(br#"{"detail":"x"}"#.to_vec()), Duration::ZERO);
    assert!(
        matches!(&step.verdict, Verdict::ShapeChanged { field } if field.starts_with("the sound")),
        "{step:?}"
    );
}

#[test]
fn a_refusal_is_not_ok() {
    let plan = sound_step(
        Err(super::refused(402, super::elevenlabs::FREE_PLAN)),
        Duration::ZERO,
    );
    assert!(matches!(plan.verdict, Verdict::Refused { .. }), "{plan:?}");
}

/// The shortest sound the vendor makes, on the product's model, played once:
/// a cent, rounded up from a tenth of one.
#[test]
fn the_sound_is_the_shortest_and_costs_a_cent() {
    let body = effect();
    assert!((body.duration_seconds - MIN_SOUND_SECONDS).abs() < f64::EPSILON);
    assert_eq!(SOUND_MILLIS, 500);
    assert_eq!(body.model_id, MODEL_ID);
    assert!(!body.looping);
    assert_eq!(sound_cents(), 1);
}
