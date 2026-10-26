//! The PNG probe (#1341): a 1K square asked for as `image/png`, whose
//! every answer is reported and only no answer fails.

use scorsese_providers::api::http::HttpError;
use scorsese_providers::live::Verdict;
use scorsese_providers::live::image::png_step;

use super::image::{JPEG_2K, PNG, drawn};
use super::refused;

/// The one note an answer leaves, after checking the step passed.
fn note(step: &scorsese_providers::live::Step) -> &str {
    assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
    assert_eq!(step.notes.len(), 1, "{step:?}");
    &step.notes[0]
}

#[test]
fn a_png_coming_back_is_reported_accepted_with_its_header() {
    let step = png_step(Ok(drawn(PNG)));
    let said = note(&step);
    assert!(
        said.starts_with("accepted") && said.contains("512x512 PNG"),
        "{said}"
    );
}

/// A refusal is the answer #1341 asks for, not a failure: the vendor's
/// words are reported so they can be recorded.
/// The width and the height are each read whole, from the header.
#[test]
fn a_wide_png_reports_its_own_size() {
    // The opening of a 1920x1080 PNG.
    let step = png_step(Ok(drawn("iVBORw0KGgoAAAANSUhEUgAAB4AAAAQ4")));
    assert!(note(&step).contains("1920x1080 PNG"), "{step:?}");
}

#[test]
fn a_refusal_is_reported_in_the_vendors_words_without_failing() {
    let supported = r#"{"error":{"code":400,"message":"Supported values: 'image/jpeg'"}}"#;
    let step = png_step(Err(refused(400, supported)));
    let said = note(&step);
    assert!(
        said.starts_with("refused") && said.contains("Supported values: 'image/jpeg'"),
        "{said}"
    );
}

#[test]
fn a_jpeg_drawn_instead_is_reported_as_the_format_ignored() {
    let step = png_step(Ok(drawn(JPEG_2K)));
    let said = note(&step);
    assert!(
        said.starts_with("ignored") && said.contains("JPEG"),
        "{said}"
    );
}

#[test]
fn words_without_a_picture_are_reported_not_failed() {
    let words: scorsese_providers::api::gemini::response::Interaction = serde_json::from_str(
        r#"{"steps": [{"type": "model_output", "content": [{"type": "text", "text": "I can't."}]}]}"#,
    )
    .unwrap();
    assert!(note(&png_step(Ok(words))).contains("I can't."));
}

#[test]
fn no_answer_at_all_fails_the_run() {
    let lost = HttpError::Unreachable {
        url: String::from("https://vendor.invalid/"),
        message: String::from("connection reset"),
    };
    let step = png_step(Err(lost));
    assert!(step.verdict.is_failure(), "{step:?}");
    assert!(matches!(step.verdict, Verdict::Unreachable { .. }));
}
