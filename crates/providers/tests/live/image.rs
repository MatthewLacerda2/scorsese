//! What Gemini's image answers mean, and what the still costs.

use scorsese_providers::api::gemini::response::Interaction;
use scorsese_providers::live::Verdict;
use scorsese_providers::live::image::{cost, picture_step};

use super::{parsed, refused};

/// A PNG header claiming 1024x1024 — what an ignored `image_size` gives back.
const PNG_1K: &str = "iVBORw0KGgoAAAANSUhEUgAABAAAAAQACAIAAAA=";

/// The opening of a JPEG.
const JPEG: &str = "/9j/4AAQSkZJRg==";

/// A reply whose one model output is this base64 picture.
fn drawn(data: &str) -> Interaction {
    serde_json::from_value(serde_json::json!({"status": "completed", "steps": [
        {"type": "model_output", "content": [{"type": "image", "data": data}]}
    ]}))
    .expect("a model output with one picture parses")
}

/// What the shape-changed field says, or a panic naming the verdict.
fn field(answer: Interaction) -> String {
    match picture_step(Ok(answer)).verdict {
        Verdict::ShapeChanged { field } => field,
        other => panic!("expected a shape change, got {other:?}"),
    }
}

#[test]
fn a_512_square_png_is_ok_and_says_so() {
    let step = picture_step(Ok(parsed("gemini/interaction.json")));
    assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
    assert!(step.notes[0].starts_with("512x512 PNG"), "{:?}", step.notes);
}

/// The guess the check exists to settle: an ignored `"512"` comes back at
/// the 1K default, billed as 0.5K, and only the picture's header shows it.
#[test]
fn a_picture_at_another_size_says_the_size_was_not_taken() {
    let said = field(drawn(PNG_1K));
    assert!(
        said.contains("image_size") && said.contains("1024x1024"),
        "{said}"
    );
}

#[test]
fn a_picture_that_is_not_a_png_or_not_base64_is_a_shape_change() {
    assert!(field(drawn(JPEG)).contains("expected a PNG"));
    assert!(field(drawn("not base64!")).contains("not base64"));
}

#[test]
fn words_without_a_picture_are_a_refusal_and_silence_is_a_shape_change() {
    let words: Interaction = serde_json::from_str(
        r#"{"steps": [{"type": "model_output", "content": [{"type": "text", "text": "I can't."}]}]}"#,
    )
    .unwrap();
    let step = picture_step(Ok(words));
    assert_eq!(
        step.verdict,
        Verdict::Refused {
            said: String::from("I can't.")
        }
    );

    let empty: Interaction = serde_json::from_str(r#"{"status": "completed"}"#).unwrap();
    assert!(field(empty).contains("output_image"));
}

#[test]
fn a_rejected_key_is_an_auth_failure_and_a_bad_request_a_refusal() {
    let bad_key = r#"{"error":{"code":400,"details":[{"reason":"API_KEY_INVALID"}]}}"#;
    let step = picture_step(Err(refused(400, bad_key)));
    assert!(
        matches!(step.verdict, Verdict::AuthFailed { .. }),
        "{step:?}"
    );

    let spelling = r#"{"error":{"code":400,"message":"Invalid image_size"}}"#;
    let step = picture_step(Err(refused(400, spelling)));
    assert!(matches!(&step.verdict, Verdict::Refused { said } if said.contains("image_size")));
}

/// Flash at 0.5K is $0.045 a picture and the prompt a fraction of a cent:
/// five cents, rounded up once.
#[test]
fn the_still_costs_five_cents() {
    assert_eq!(cost(), 5);
}
