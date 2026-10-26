//! Effort is part of the conversation (#1443): four levels, a top-level
//! setting that never moves, and a change written into the history where a
//! turn at another level opens.

use std::collections::HashMap;

use scorsese_providers::chat::{Effort, Kept, Message, Model, Request, anthropic, freeze, replay};
use serde_json::json;

/// A request on `model` carrying `messages`, each frozen for it.
fn request(model: Model, messages: &[Message]) -> Request {
    Request {
        model,
        system: "s".into(),
        tools: vec![],
        messages: messages
            .iter()
            .map(|m| freeze(model, m).expect("a message freezes"))
            .collect(),
    }
}

#[test]
fn four_levels_read_back_from_the_names_they_are_sent_as() {
    let names: Vec<&str> = Effort::ALL.iter().map(|e| e.as_str()).collect();
    assert_eq!(names, ["low", "medium", "high", "xhigh"]);
    for effort in Effort::ALL {
        assert_eq!(Effort::from_name(effort.as_str()), Some(effort));
    }
    assert_eq!(Effort::from_name("max"), None, "not offered");
    assert_eq!(Effort::from_name("High"), None);
}

#[test]
fn the_top_level_setting_is_the_baseline_whatever_the_turn_runs_at() {
    let low = Effort::Low.change(&[]).expect("low is not the baseline");
    let request = request(Model::ClaudeOpus55, &[low, Message::user("nudge it")]);
    let body = serde_json::to_value(anthropic::laid_out(&request)).expect("a request serialises");
    assert_eq!(body["output_config"], json!({"effort": "high"}));
    assert_eq!(
        body["messages"][0],
        json!({"role": "system", "content": [], "output_config": {"effort": "low"}})
    );
    assert_eq!(request.effort(), Effort::Low);
}

#[test]
fn a_turn_opens_with_a_change_only_when_its_level_is_not_the_one_in_effect() {
    assert_eq!(Effort::BASELINE, Effort::High);
    assert_eq!(Effort::High.change(&[]), None, "the baseline needs none");
    let low = Effort::Low.change(&[]).expect("low is not the baseline");
    let history = [low.clone(), Message::user("a"), Message::assistant("b")];
    assert_eq!(Effort::in_effect(&history), Effort::Low);
    assert_eq!(Effort::Low.change(&history), None, "still in effect");
    assert_eq!(
        Effort::High.change(&history),
        Some(Message::Effort {
            effort: Effort::High
        }),
        "back to the baseline is a change too"
    );
    let request = request(Model::ClaudeSonnet55, &history);
    assert_eq!(request.effort(), Effort::Low);
}

#[test]
fn a_change_survives_the_record_and_a_change_of_model() {
    let change = Message::Effort {
        effort: Effort::Xhigh,
    };
    assert_eq!(
        serde_json::to_value(&change).expect("a message serialises"),
        json!({"role": "effort", "effort": "xhigh"})
    );
    let frozen = freeze(Model::ClaudeOpus55, &change).expect("a change freezes");
    assert_eq!(
        anthropic::read(std::slice::from_ref(&frozen), &mut HashMap::new()),
        std::slice::from_ref(&change)
    );
    let kept = Kept {
        model: Model::ClaudeOpus55.id().to_owned(),
        native: vec![frozen.clone()],
        record: vec![change],
    };
    let replayed = replay(Model::ClaudeFable51, &[kept]).expect("a history replays");
    assert_eq!(
        replayed[0].get(),
        frozen.get(),
        "the same bytes on every model"
    );
}
