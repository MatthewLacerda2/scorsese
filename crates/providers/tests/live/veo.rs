//! What Veo's answers mean, and what the shot costs.

use scorsese_providers::api::http::HttpError;
use scorsese_providers::api::veo::Model;
use scorsese_providers::api::veo::response::{ModelInfo, Operation};
use scorsese_providers::live::veo::{cost, model_step, operation_step, submitted_step, video_step};
use scorsese_providers::live::{Options, Verdict};

use super::{parsed, refused};

/// Google's answer to a key it does not accept — written from its error
/// reference, not captured: a `400`, with the reason in the details.
const BAD_KEY: &str = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.",
    "status":"INVALID_ARGUMENT","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo",
    "reason":"API_KEY_INVALID","domain":"googleapis.com"}]}}"#;

#[test]
fn a_model_that_is_served_for_long_running_prediction_is_ok() {
    let step = model_step(Model::Fast, Ok(parsed("veo/model.json")));
    assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
}

#[test]
fn a_renamed_model_or_a_missing_method_is_a_shape_change_naming_the_field() {
    let mut info: ModelInfo = parsed("veo/model.json");
    let renamed = model_step(Model::Lite, Ok(info.clone()));
    assert!(
        matches!(&renamed.verdict, Verdict::ShapeChanged { field } if field.starts_with("name"))
    );

    info.supported_generation_methods = vec![String::from("generateContent")];
    let step = model_step(Model::Fast, Ok(info));
    assert!(
        matches!(&step.verdict, Verdict::ShapeChanged { field } if field.contains("predictLongRunning"))
    );
}

/// Google says *bad key* with a 400, not a 401 — read as the status alone it
/// would be reported as a malformed request.
#[test]
fn a_rejected_key_is_an_auth_failure_even_at_400() {
    let step = model_step(Model::Fast, Err(refused(400, BAD_KEY)));
    assert!(
        matches!(step.verdict, Verdict::AuthFailed { .. }),
        "{step:?}"
    );

    let retired = model_step(Model::Fast, Err(refused(404, r#"{"error":{"code":404}}"#)));
    assert!(
        matches!(retired.verdict, Verdict::Refused { .. }),
        "{retired:?}"
    );
}

#[test]
fn an_unreadable_reply_is_a_shape_change_carrying_serdes_words() {
    let error = HttpError::Unreadable {
        url: String::new(),
        message: String::from("missing field `name` at line 1 column 2"),
    };
    let (step, ticket) = submitted_step(Err(error));
    assert!(matches!(&step.verdict, Verdict::ShapeChanged { field } if field.contains("`name`")));
    assert_eq!(ticket, None);
}

#[test]
fn a_submit_hands_back_the_operation_to_poll() {
    let (step, ticket) = submitted_step(Ok(parsed("veo/submitted.json")));
    assert_eq!(step.verdict, Verdict::Ok);
    assert!(ticket.unwrap().contains("/operations/"));
}

#[test]
fn an_operation_is_waited_on_until_it_is_done_and_then_judged() {
    assert!(operation_step(Ok(parsed("veo/running.json"))).is_none());

    let (step, uri) = operation_step(Ok(parsed("veo/done.json"))).unwrap();
    assert_eq!(step.verdict, Verdict::Ok);
    assert!(uri.unwrap().contains(":download"));

    let (failed, _) = operation_step(Ok(parsed("veo/failed.json"))).unwrap();
    assert!(
        matches!(failed.verdict, Verdict::Refused { .. }),
        "{failed:?}"
    );
}

/// Done with nothing in it is the vendor's shape having moved; done with an
/// empty response is what a filtered shot looks like.
#[test]
fn a_done_operation_without_a_video_is_told_apart() {
    let bare: Operation = serde_json::from_str(r#"{"done": true}"#).unwrap();
    let (step, _) = operation_step(Ok(bare)).unwrap();
    assert!(
        matches!(&step.verdict, Verdict::ShapeChanged { field } if field.contains("generatedSamples"))
    );

    let empty: Operation =
        serde_json::from_str(r#"{"done": true, "response": {"generateVideoResponse": {}}}"#)
            .unwrap();
    let (step, _) = operation_step(Ok(empty)).unwrap();
    assert!(matches!(step.verdict, Verdict::Refused { .. }), "{step:?}");
}

#[test]
fn the_download_must_be_an_mp4() {
    let mp4 = b"\0\0\0\x18ftypmp42rest".to_vec();
    assert_eq!(video_step(Ok(mp4)).verdict, Verdict::Ok);
    let json = video_step(Ok(br#"{"error":{}}"#.to_vec()));
    assert!(matches!(&json.verdict, Verdict::ShapeChanged { field } if field.contains("JSON")));
}

/// The shot is the cheapest Veo sells, and nothing is spent without asking.
#[test]
fn the_shot_costs_twenty_cents_and_only_when_asked_for() {
    assert_eq!(cost(&Options::default()), 0);
    let asked = Options {
        include_veo: true,
        ..Options::default()
    };
    assert_eq!(cost(&asked), 20);
}
