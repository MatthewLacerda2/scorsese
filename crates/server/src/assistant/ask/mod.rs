//! Asking the user a question mid-edit (#710): the one function the assistant
//! declares to its model that is not a tool.
//!
//! The model sometimes reaches a choice only the person can make — the music
//! ends four seconds before the video: loop it, fade it, or stretch the last
//! clip? — halfway through a turn whose rest depends on the answer. Ending the
//! turn with the question would make the answer a fresh turn that has to
//! rebuild where the edit was. So the loop offers the model `ask_user`
//! beside the tools: one short question and two to four options. Calling it
//! **pauses the turn** (`asking`) with its plan intact, the chat panel shows
//! the question as a card, and the answer resumes **the same turn** as that
//! call's result (`answer`). Free text is always allowed beside the options,
//! and a chat message written instead of an answer is the answer.
//!
//! **Not a tool.** `ask_user` is declared here, appended after the
//! registry's tools, and handled here; it is never in
//! [`Toolbox`](crate::tools::Toolbox), so web MCP never lists it. A user's
//! own Gemini or GPT over web MCP asks its user its own way (the module doc of
//! [`super`]: what the assistant adds is around the tools, never one of them).
//!
//! **One at a time.** The call must be alone in its reply: a reply that also
//! calls tools has every other call run and `ask_user` refused with a line
//! saying so, and the model asks again once it has their results. Pausing
//! between results that would have to be sent together is what that avoids.
//!
//! **Waiting is free, and never expires.** A paused turn holds no process,
//! no money and no token: nothing is charged until the answer resumes it, and
//! the calls it makes then are charged and recorded like any other. A late
//! answer costs at most one uncached call, the same as any late reply, so
//! there is nothing a deadline would protect. What ends a question unanswered
//! is the user: Stop, or a new conversation.
//!
//! **Pictures too** (#901, `pick`): `pick_stock` is the same pause with stock
//! results for options, picked in a modal instead of a card.

mod answer;
mod pick;

pub use answer::set_aside;
pub use answer::{Answering, answer};

use scorsese_providers::chat::{Call, Message, Part, ResultPart, Tool};
use serde_json::{Value, json};

use super::store::QuestionView;
use crate::db::UserId;
use crate::http::AppState;

/// What a reply's calls ask of the person.
pub(super) enum Asked {
    /// Nothing: its calls run.
    Nothing,
    /// The turn pauses on this.
    Pause(QuestionView),
    /// A picker alone in its reply that cannot be shown: this result instead,
    /// and the turn goes on.
    Refused(Part),
}

/// Every function the loop declares beside the registry's tools.
pub(super) fn functions() -> [Tool; 2] {
    [tool(), pick::tool()]
}

/// Whether `name` is one of [`functions`] — a call the loop answers itself.
pub(super) fn is_ours(name: &str) -> bool {
    name == NAME || name == pick::NAME
}

/// What `calls`, made in `user`'s turn whose messages so far are `record`,
/// ask of the person: a well-formed question or picker alone pauses the turn.
pub(super) async fn asked(
    state: &AppState,
    user: UserId,
    record: &[Message],
    calls: &[Call],
) -> Asked {
    match calls {
        [call] if call.name == pick::NAME => {
            match pick::offer(state, user, record, &call.input).await {
                Ok(picker) => Asked::Pause(picker),
                Err(why) => Asked::Refused(pick::refused(call, why)),
            }
        }
        _ => alone(calls).map_or(Asked::Nothing, Asked::Pause),
    }
}

/// What the model calls it by.
pub(super) const NAME: &str = "ask_user";

/// How many options a question offers, at least and at most.
const OPTIONS: std::ops::RangeInclusive<usize> = 2..=4;

/// `ask_user` as the model is offered it. Constant, like the system prompt:
/// it is part of the cached prefix.
fn tool() -> Tool {
    Tool {
        name: NAME.to_owned(),
        description: "Ask the person one short question mid-edit and wait for the answer, \
            which comes back as this call's result. Only for a choice that changes what you do \
            next and that you cannot reasonably make yourself — never to confirm a step, and \
            never about money (a paid tool's quote already asks). Call it alone, with no other \
            call in the same reply."
            .to_owned(),
        schema: json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question, in one plain sentence in the person's \
                        language."
                },
                "options": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": OPTIONS.start(),
                    "maxItems": OPTIONS.end(),
                    "description": "Two to four short answers to pick from. The person can \
                        always write their own instead."
                }
            },
            "required": ["question", "options"]
        }),
    }
}

/// The question `calls` asks, when it is a well-formed `ask_user` alone.
fn alone(calls: &[Call]) -> Option<QuestionView> {
    match calls {
        [call] if call.name == NAME => question(&call.input).ok(),
        _ => None,
    }
}

/// The result an `ask_user` or `pick_stock` call gets when it cannot pause
/// the turn: badly formed, or not alone in its reply.
pub(super) fn refused(call: &Call) -> Part {
    if call.name == pick::NAME {
        let why = match pick::read(&call.input) {
            Err(why) => why,
            Ok(_) => format!(
                "Not shown: {} must be the only call in its reply. Your other calls ran; show \
                 it again on its own if you still need to.",
                pick::NAME
            ),
        };
        return pick::refused(call, why);
    }
    let why = match question(&call.input) {
        Err(why) => why,
        Ok(_) => "Not asked: ask_user must be the only call in its reply. Your other calls \
                  ran; ask again on its own if you still need to."
            .to_owned(),
    };
    Part::Result {
        call: call.id.clone(),
        name: call.name.clone(),
        content: vec![ResultPart::Text { text: why }],
        is_error: true,
    }
}

/// The result that resumes a turn: what the person answered.
pub(super) fn answered(call: &Call, answer: &str) -> Part {
    Part::Result {
        call: call.id.clone(),
        name: call.name.clone(),
        content: vec![ResultPart::Text {
            text: format!("The person answered: {answer}"),
        }],
        is_error: false,
    }
}

/// `input` read as a question, or why it is not one, in words for the model.
fn question(input: &Value) -> Result<QuestionView, String> {
    let asked = input
        .get("question")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if asked.is_empty() {
        return Err("Not asked: `question` is missing or empty.".into());
    }
    let options: Vec<String> = input
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            let words = options.iter().filter_map(Value::as_str).map(str::trim);
            words
                .filter(|word| !word.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let mut distinct = options.clone();
    distinct.sort();
    distinct.dedup();
    if !OPTIONS.contains(&options.len()) || distinct.len() != options.len() {
        return Err(format!(
            "Not asked: `options` must be {} to {} different, non-empty answers.",
            OPTIONS.start(),
            OPTIONS.end()
        ));
    }
    Ok(QuestionView {
        question: asked.to_owned(),
        options,
        answer: None,
        candidates: Vec::new(),
        picked: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(input: Value) -> Call {
        Call {
            id: "toolu_1".into(),
            name: NAME.into(),
            input,
        }
    }

    #[test]
    fn a_question_with_two_to_four_options_pauses() {
        let asked = json!({ "question": " Loop or fade? ", "options": ["loop", "fade"] });
        let question = alone(&[call(asked)]).expect("a question");
        assert_eq!(question.question, "Loop or fade?");
        assert_eq!(question.options, ["loop", "fade"]);
        assert_eq!(question.answer, None);
        assert!(!question.is_picker());
    }

    #[test]
    fn too_few_too_many_or_repeated_options_are_refused() {
        for options in [
            json!(["one"]),
            json!(["a", "b", "c", "d", "e"]),
            json!(["a", "a"]),
        ] {
            let asked = call(json!({ "question": "Which?", "options": options }));
            assert_eq!(alone(std::slice::from_ref(&asked)), None, "{options}");
            let Part::Result {
                is_error, content, ..
            } = refused(&asked)
            else {
                panic!("a result");
            };
            assert!(is_error);
            assert!(format!("{content:?}").contains("options"), "{content:?}");
        }
        assert_eq!(alone(&[call(json!({ "options": ["a", "b"] }))]), None);
    }

    #[test]
    fn a_question_beside_another_call_is_refused_as_not_alone() {
        let asked = call(json!({ "question": "Which?", "options": ["a", "b"] }));
        let other = Call {
            id: "toolu_2".into(),
            name: "project_describe".into(),
            input: json!({}),
        };
        assert_eq!(alone(&[asked.clone(), other]), None);
        assert!(format!("{:?}", refused(&asked)).contains("only call"));
    }

    #[test]
    fn its_declaration_is_described_like_a_registry_tool() {
        let tool = tool();
        assert_eq!(tool.name, NAME);
        for argument in ["question", "options"] {
            let said = tool.schema["properties"][argument]["description"].as_str();
            assert!(said.unwrap_or_default().len() > 15, "{argument}");
        }
    }
}
