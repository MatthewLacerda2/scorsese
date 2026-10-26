//! The PNG probe (#1341): a 1K square asked for as `image/png`, whose
//! every answer is reported and only no answer fails.

use scorsese_providers::api::http::HttpError;
use scorsese_providers::live::Verdict;
use scorsese_providers::live::image::{png_cents, png_step, still_cents};

use super::image::{JPEG_2K, PNG, drawn};
use super::refused;

/// The one note an answer leaves, after checking the step passed.
fn note(step: &scorsese_providers::live::Step) -> &str {
    assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
    assert_eq!(step.notes.len(), 1, "{step:?}");
    &step.notes[0]
}

/// The opening of a 1024x1024 PNG: the size a 1K square is drawn at.
const PNG_1K: &str = "iVBORw0KGgoAAAANSUhEUgAABAAAAAQA";

#[test]
fn a_png_coming_back_is_reported_accepted_with_its_header() {
    let step = png_step(Ok(drawn(PNG_1K)));
    let said = note(&step);
    assert!(
        said.starts_with("accepted")
            && said.contains("1024x1024 PNG")
            && said.contains("can switch"),
        "{said}"
    );
}

/// A PNG at another size took the format and not the size: #1341 cannot
/// switch on it.
#[test]
fn a_png_at_another_size_says_so_and_not_that_it_can_switch() {
    let said = note(&png_step(Ok(drawn(PNG)))).to_owned();
    assert!(
        said.contains("512x512 PNG") && said.contains("1024x1024") && !said.contains("can switch"),
        "{said}"
    );
}

/// The width and the height are each read whole, from the header.
#[test]
fn a_wide_png_reports_its_own_size() {
    // The opening of a 1920x1080 PNG.
    let step = png_step(Ok(drawn("iVBORw0KGgoAAAANSUhEUgAAB4AAAAQ4")));
    assert!(note(&step).contains("1920x1080 PNG"), "{step:?}");
}

/// A refusal is the answer #1341 asks for, not a failure: the vendor's
/// words are reported so they can be recorded.
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

/// A 429 or a 5xx says nothing about `image/png`, so it is never called a
/// refusal #1341 could be closed on.
#[test]
fn a_transient_error_is_inconclusive_with_its_status() {
    for status in [429, 500, 503] {
        let said = note(&png_step(Err(refused(status, "try again later")))).to_owned();
        assert!(
            said.starts_with("inconclusive") && said.contains(&status.to_string()),
            "{said}"
        );
    }
}

/// A rejected key still fails the run, whatever the probe reports.
#[test]
fn a_rejected_key_fails_the_run() {
    let step = png_step(Err(refused(403, "API_KEY_INVALID")));
    assert!(
        matches!(step.verdict, Verdict::AuthFailed { .. }),
        "{step:?}"
    );
}

/// A picture of either kind was drawn, and billed, and so was a 2xx reply
/// that would not parse, as the still and ElevenLabs count it; a refusal or
/// the model's words alone were not.
#[test]
fn the_probe_is_counted_spent_whenever_the_vendor_drew() {
    let unreadable = HttpError::Unreadable {
        url: String::from("https://vendor.invalid/"),
        message: String::from("expected value at line 1"),
    };
    assert_eq!(png_cents(&Err(unreadable)), still_cents());
    assert_eq!(png_cents(&Ok(drawn(PNG_1K))), still_cents());
    assert_eq!(png_cents(&Ok(drawn(JPEG_2K))), still_cents());
    assert_eq!(
        png_cents(&Err(refused(400, "Supported values: 'image/jpeg'"))),
        0
    );
    let words: scorsese_providers::api::gemini::response::Interaction = serde_json::from_str(
        r#"{"steps": [{"type": "model_output", "content": [{"type": "text", "text": "I can't."}]}]}"#,
    )
    .unwrap();
    assert_eq!(png_cents(&Ok(words)), 0);
    let empty: scorsese_providers::api::gemini::response::Interaction =
        serde_json::from_str(r#"{"status": "completed", "steps": []}"#).unwrap();
    assert_eq!(
        png_cents(&Ok(empty)),
        still_cents(),
        "neither picture nor words is billed"
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
