//! The seam's edges: what is worth retrying, and the models offered.

use scorsese_providers::chat::{ChatError, ClaudeError, Model};

#[test]
fn only_a_busy_or_broken_vendor_is_worth_asking_again() {
    assert!(ChatError::from(ClaudeError::Cut).retryable());
    assert!(!ChatError::from(ClaudeError::Arguments("x".into())).retryable());
}

#[test]
fn a_conversation_over_the_byte_or_the_token_limit_is_too_large_and_nothing_else_is() {
    use scorsese_providers::api::http::HttpError;
    let refused = |status: u16, body: &str| {
        ChatError::from(ClaudeError::Http(HttpError::Refused {
            url: "https://api.anthropic.com/v1/messages".into(),
            status,
            body: body.into(),
        }))
    };
    let tokens = r#"{"type": "error", "error": {"type": "invalid_request_error", "message": "prompt is too long: 215034 tokens > 200000 maximum"}}"#;
    assert!(refused(400, tokens).too_large(), "the token wall (#1435)");
    assert!(refused(413, "").too_large(), "the byte wall");
    let streamed = ClaudeError::Api {
        kind: "invalid_request_error".into(),
        message: "prompt is too long: 215034 tokens > 200000 maximum".into(),
    };
    assert!(ChatError::from(streamed).too_large());
    let other = r#"{"type": "error", "error": {"type": "invalid_request_error", "message": "messages: roles must alternate"}}"#;
    assert!(!refused(400, other).too_large());
    assert!(!refused(400, "not json").too_large());
    assert!(!refused(529, tokens).too_large());
}

#[test]
fn exactly_sonnet_opus_then_fable_are_offered_and_keep_their_cache_an_hour() {
    let offered: Vec<(&str, &str)> = Model::ALL.iter().map(|m| (m.id(), m.label())).collect();
    assert_eq!(
        offered,
        [
            ("claude-sonnet-5-5", "Claude Sonnet 5.5"),
            ("claude-opus-5-5", "Claude Opus 5.5"),
            ("claude-fable-5-1", "Claude Fable 5.1")
        ]
    );
    assert_eq!(Model::DEFAULT, Model::ClaudeOpus55, "#1444");
    for model in Model::ALL {
        assert_eq!(Model::from_id(model.id()), Some(model));
        assert_eq!(model.cache_lifetime().as_secs(), 3_600);
    }
    assert_eq!(Model::from_id("gemini-3.8-flash"), None, "cut in #1220");
}

#[test]
fn a_picture_stored_as_png_before_it_named_its_type_still_reads_and_a_jpeg_names_its_own() {
    use scorsese_providers::chat::ResultPart;
    let old: ResultPart = serde_json::from_str(r#"{"type": "png", "data": "UE5H"}"#).unwrap();
    let png = ResultPart::Image {
        media_type: "image/png".into(),
        data: "UE5H".into(),
    };
    assert_eq!(old, png, "a row written before #1435");
    let jpeg = ResultPart::Image {
        media_type: "image/jpeg".into(),
        data: "/9j/".into(),
    };
    let stored = serde_json::to_value(&jpeg).unwrap();
    assert_eq!(
        stored,
        serde_json::json!({"type": "image", "media_type": "image/jpeg", "data": "/9j/"})
    );
    assert_eq!(serde_json::from_value::<ResultPart>(stored).unwrap(), jpeg);
}
