//! The seam's edges: what is worth retrying, the models offered, and how an
//! effort reaches the wire.

use scorsese_providers::chat::{ChatError, ClaudeError, Effort, Model, Request, anthropic};

#[test]
fn only_a_busy_or_broken_vendor_is_worth_asking_again() {
    assert!(ChatError::from(ClaudeError::Cut).retryable());
    assert!(!ChatError::from(ClaudeError::Arguments("x".into())).retryable());
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
fn an_effort_reaches_the_wire_as_the_name_it_reads_back_from() {
    for effort in [Effort::Low, Effort::Medium, Effort::High] {
        let name = effort.as_str();
        assert_eq!(Effort::from_name(name), Some(effort));
        let request = Request {
            model: Model::ClaudeOpus55,
            effort,
            system: "s".into(),
            tools: vec![],
            messages: vec![],
        };
        let body = serde_json::to_value(anthropic::laid_out(&request)).unwrap();
        assert_eq!(body["output_config"]["effort"], name, "{body}");
    }
    assert_eq!(Effort::from_name("xhigh"), None);
    assert_eq!(Effort::from_name("High"), None);
}
