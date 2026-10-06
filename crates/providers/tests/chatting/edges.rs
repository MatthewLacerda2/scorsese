//! The seam's edges: a request sent to the wrong vendor, what is worth
//! retrying, how roles alternate on Gemini, and the stream's odd shapes.

use scorsese_providers::api::gemini::chat::chunks;
use scorsese_providers::api::gemini::chat::request::Part as WirePart;
use scorsese_providers::chat::gemini::{Gemini, GeminiError, laid_out};
use scorsese_providers::chat::{Chat, ChatError, Effort, Message, Model, Part, Request, freeze};
use scorsese_providers::claude::Anthropic;
use scorsese_providers::credentials::Secret;
use serde_json::{Value, json};

fn request(model: Model, messages: &[Message]) -> Request {
    Request {
        model,
        effort: Effort::Low,
        system: "s".into(),
        tools: vec![],
        messages: messages
            .iter()
            .map(|m| freeze(model, m).expect("the test setup holds"))
            .collect(),
    }
}

#[test]
fn a_model_sent_to_the_other_vendor_is_refused_before_any_call() {
    let key = Secret::new("never-used");
    let to_google = Gemini::new(&key).reply(&request(Model::ClaudeOpus55, &[]), &mut |_| {});
    assert!(matches!(to_google, Err(ChatError::Misrouted(_))));
    let to_anthropic = Anthropic::new(&key).reply(&request(Model::GeminiFlash38, &[]), &mut |_| {});
    let error = to_anthropic.unwrap_err();
    assert!(error.to_string().contains("Gemini 3.8 Flash"), "{error}");
    assert!(!error.retryable());
}

#[test]
fn only_a_busy_or_broken_vendor_is_worth_asking_again() {
    let api = |status: &str| GeminiError::Api {
        status: status.into(),
        message: "m".into(),
    };
    assert!(ChatError::from(api("UNAVAILABLE")).retryable());
    assert!(!ChatError::from(api("INVALID_ARGUMENT")).retryable());
    assert!(!GeminiError::Unwritable("x".into()).retryable());
    assert!(ChatError::from(scorsese_providers::chat::ClaudeError::Cut).retryable());
}

#[test]
fn gemini_sides_alternate_and_are_never_joined_across() {
    let messages = [
        Message::user("a"),
        Message::assistant("b"),
        Message::user("c"),
    ];
    let body = laid_out(&request(Model::GeminiFlash38, &messages)).unwrap();
    let roles: Vec<Option<String>> = body.contents.iter().map(|c| c.role.clone()).collect();
    assert_eq!(
        roles,
        [
            Some("user".into()),
            Some("model".into()),
            Some("user".into())
        ]
    );
    assert!(serde_json::to_value(&body).unwrap().get("tools").is_none());
}

#[test]
fn a_call_with_no_arguments_has_an_empty_object_and_plain_words_say_no_thought() {
    let part: WirePart =
        serde_json::from_value(json!({"functionCall": {"name": "icons"}})).unwrap();
    assert_eq!(part.function_call.unwrap().args, json!({}));
    let words = serde_json::to_value(WirePart {
        text: Some("hi".into()),
        ..WirePart::default()
    })
    .unwrap();
    assert_eq!(words, json!({"text": "hi"}));
}

#[test]
fn data_split_over_lines_is_one_event_and_blank_lines_between_are_nothing() {
    let stream = "\n\ndata: {\"responseId\":\ndata: \"r1\"}\n\n\ndata: {\"responseId\":\"r2\"}\n";
    let ids: Vec<Value> = chunks(stream.as_bytes())
        .map(|chunk| json!(chunk.unwrap().response_id))
        .collect();
    assert_eq!(ids, [json!("r1"), json!("r2")]);
}

#[test]
fn every_model_is_labelled_and_keeps_its_cache_an_hour() {
    let labels: Vec<&str> = Model::ALL.iter().map(|m| m.label()).collect();
    assert_eq!(
        labels,
        [
            "Gemini 3.8 Flash",
            "Gemini 3.5 Flash Lite",
            "Claude Opus 5.5",
            "Claude Sonnet 5.5"
        ]
    );
    for model in Model::ALL {
        assert_eq!(model.cache_lifetime().as_secs(), 3_600);
    }
}

#[test]
fn a_neutral_text_part_on_the_model_side_is_kept_and_an_empty_one_is_not() {
    let message = Message::Assistant {
        content: vec![Part::Text { text: "x".into() }],
    };
    let frozen: Value =
        serde_json::from_str(freeze(Model::GeminiFlash38, &message).unwrap().get()).unwrap();
    assert_eq!(frozen, json!({"role": "model", "parts": [{"text": "x"}]}));
}

#[test]
fn an_effort_reaches_both_wires_as_the_name_it_reads_back_from() {
    use scorsese_providers::chat::anthropic;
    for effort in [Effort::Low, Effort::Medium, Effort::High] {
        let name = effort.as_str();
        assert_eq!(Effort::from_name(name), Some(effort));
        let mut claude = request(Model::ClaudeOpus55, &[]);
        claude.effort = effort;
        let body = serde_json::to_value(anthropic::laid_out(&claude)).unwrap();
        assert_eq!(body["output_config"]["effort"], name, "{body}");
        let mut gemini = request(Model::GeminiFlash38, &[]);
        gemini.effort = effort;
        let body = serde_json::to_value(laid_out(&gemini).unwrap()).unwrap();
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            name
        );
    }
    assert_eq!(Effort::from_name("xhigh"), None);
    assert_eq!(Effort::from_name("High"), None);
}
