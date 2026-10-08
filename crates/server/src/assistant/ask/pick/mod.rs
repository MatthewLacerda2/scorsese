//! Showing the user a few stock candidates and letting them pick (#901): the
//! mid-edit question (#710) with pictures for options.
//!
//! "Which sunrise" is often taste, and the model cannot know the user's. So
//! beside `ask_user` the loop declares `pick_stock`: a one-line question and
//! two to eight results the turn's own `stock_search` calls listed. Calling
//! it pauses the turn exactly as a question does — the same `asking` state,
//! stored as a [`QuestionView`] whose `candidates` are filled — and the chat
//! panel opens a picker instead of a card. The user picks one or more, or
//! none, or writes something instead; whatever they picked is imported
//! (`taken`) and the turn resumes with the new asset ids as the call's
//! result.
//!
//! **Only what a search showed.** Every candidate must be one this turn's
//! `stock_search` replies listed and still in the user's 24-hour stock cache
//! (`offer`): the cache is the user's, shared by every turn and project, so
//! being in it says only that *some* search found it. A model naming an id
//! it never saw — or a stale one — is refused, and the turn goes on.
//!
//! **Only what was picked is downloaded.** The picker shows each candidate
//! from the source's own preview and file URLs, which Pixabay allows for
//! showing search results; a project never stores a URL.
//!
//! **Not a tool**, for the reason `ask_user` is not one (the module doc of
//! [`super`]): a user's own client over web MCP shows candidates its own way.

mod offer;
mod taken;

pub(in crate::assistant) use offer::offer;
pub(in crate::assistant) use taken::{chosen, result};

use scorsese_providers::chat::{Call, Part, ResultPart, Tool};
use scorsese_providers::stock::{Choice, Medium};
use scorsese_render::Resolution;
use serde_json::{Value, json};

/// What the model calls it by.
pub(in crate::assistant) const NAME: &str = "pick_stock";

/// How many candidates a picker offers, at least and at most.
const CANDIDATES: std::ops::RangeInclusive<usize> = 2..=8;

/// `pick_stock` as the model is offered it. Constant, like the system
/// prompt: it is part of the cached prefix.
pub(in crate::assistant) fn tool() -> Tool {
    Tool {
        name: NAME.to_owned(),
        description: "Show the person stock results your stock_search calls in this turn \
            listed, as pictures they can enlarge and play, and let them pick one or more — or \
            none. Only when several are equally good or you are unsure between them, so the \
            choice is their taste; when one serves or one is clearly better, stock_import it \
            yourself and say so. What they pick is imported for you, and its asset ids come back \
            as this call's result: do not stock_import it again. Call it alone, with no other \
            call in the same reply."
            .to_owned(),
        schema: json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "What they are choosing, in one plain sentence in the \
                        person's language: \"Which sunrise do you like for the opening?\""
                },
                "candidates": {
                    "type": "array",
                    "minItems": CANDIDATES.start(),
                    "maxItems": CANDIDATES.end(),
                    "description": "Two to eight results to show, each as a stock_search reply \
                        named it: its kind and its id.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "kind": {
                                "type": "string",
                                "enum": ["video", "image"],
                                "description": "video or image, as the search was for."
                            },
                            "id": {
                                "type": "integer",
                                "description": "The id the stock_search reply listed."
                            }
                        },
                        "required": ["kind", "id"]
                    }
                },
                "resolution": {
                    "type": "string",
                    "description": "The size the video will be rendered at, e.g. 1080x1920 for \
                        a vertical cut; default 1920x1080. What they pick is downloaded at the \
                        smallest size that fills it."
                }
            },
            "required": ["question", "candidates"]
        }),
    }
}

/// What a `pick_stock` call asks for, read and checked on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::assistant) struct Asked {
    /// The question.
    pub(in crate::assistant) question: String,
    /// The candidates, in the order to show them.
    pub(in crate::assistant) candidates: Vec<Choice>,
    /// The frame what is picked must fill.
    pub(in crate::assistant) frame: Resolution,
}

/// `input` read as a picker, or why it is not one, in words for the model.
pub(in crate::assistant) fn read(input: &Value) -> Result<Asked, String> {
    let question = input
        .get("question")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if question.is_empty() {
        return Err("Not shown: `question` is missing or empty.".into());
    }
    let listed = input.get("candidates").and_then(Value::as_array);
    let mut candidates = Vec::new();
    for one in listed.into_iter().flatten() {
        let medium = match one.get("kind").and_then(Value::as_str) {
            Some("video") => Medium::Video,
            Some("image") => Medium::Image,
            _ => return Err("Not shown: each candidate's `kind` is video or image.".into()),
        };
        let id = one
            .get("id")
            .and_then(Value::as_u64)
            .ok_or("Not shown: each candidate needs the `id` a stock_search reply listed.")?;
        let choice = Choice { medium, id };
        if candidates.contains(&choice) {
            return Err(format!(
                "Not shown: {} {id} is listed twice.",
                medium.word()
            ));
        }
        candidates.push(choice);
    }
    if !CANDIDATES.contains(&candidates.len()) {
        return Err(format!(
            "Not shown: `candidates` must be {} to {} results.",
            CANDIDATES.start(),
            CANDIDATES.end()
        ));
    }
    let frame = match input
        .get("resolution")
        .and_then(Value::as_str)
        .map(str::trim)
    {
        None | Some("") => Resolution::HD,
        Some(text) => text
            .parse()
            .map_err(|problem| format!("Not shown: resolution: {problem}"))?,
    };
    Ok(Asked {
        question: question.to_owned(),
        candidates,
        frame,
    })
}

/// What a `pick_stock` call that cannot pause the turn gets instead.
pub(in crate::assistant) fn refused(call: &Call, why: String) -> Part {
    Part::Result {
        call: call.id.clone(),
        name: call.name.clone(),
        content: vec![ResultPart::Text { text: why }],
        is_error: true,
    }
}

/// The key a candidate is picked by: unique within a picker, and naming its
/// source so another source's ids can never collide with it.
pub(in crate::assistant) fn key(choice: Choice) -> String {
    format!("pixabay-{}-{}", choice.medium.word(), choice.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(candidates: Value) -> Result<Asked, String> {
        read(&json!({ "question": " Which sunrise? ", "candidates": candidates }))
    }

    #[test]
    fn two_to_eight_distinct_results_are_a_picker() {
        let read = asked(json!([{ "kind": "video", "id": 3 }, { "kind": "image", "id": 3 }]));
        let read = read.expect("a picker");
        assert_eq!(read.question, "Which sunrise?");
        assert_eq!(read.frame, Resolution::HD);
        let video = Choice {
            medium: Medium::Video,
            id: 3,
        };
        assert_eq!(
            read.candidates,
            [
                video,
                Choice {
                    medium: Medium::Image,
                    id: 3
                }
            ]
        );
    }

    #[test]
    fn too_few_too_many_repeated_or_unnamed_results_are_refused() {
        let one = json!({ "kind": "video", "id": 1 });
        let nine: Vec<Value> = (1..=9)
            .map(|id| json!({ "kind": "video", "id": id }))
            .collect();
        for candidates in [
            json!([one]),
            json!(nine),
            json!([one, one]),
            json!([one, { "kind": "audio", "id": 2 }]),
            json!([one, { "kind": "video" }]),
        ] {
            let refused = asked(candidates.clone()).expect_err("refused");
            assert!(refused.starts_with("Not shown"), "{candidates}: {refused}");
        }
        assert!(read(&json!({ "candidates": [one, { "kind": "video", "id": 2 }] })).is_err());
    }

    #[test]
    fn a_resolution_is_the_frame_to_fill() {
        let two = json!([{ "kind": "video", "id": 1 }, { "kind": "video", "id": 2 }]);
        let input = json!({ "question": "Which?", "candidates": two, "resolution": "1080x1920" });
        let frame = read(&input).expect("a picker").frame;
        assert_eq!((frame.width(), frame.height()), (1080, 1920));
        let input = json!({ "question": "Which?", "candidates": two, "resolution": "huge" });
        assert!(read(&input).is_err());
    }

    #[test]
    fn its_declaration_is_described_like_a_registry_tool() {
        let tool = tool();
        assert_eq!(tool.name, NAME);
        for argument in ["question", "candidates", "resolution"] {
            let said = tool.schema["properties"][argument]["description"].as_str();
            assert!(said.unwrap_or_default().len() > 15, "{argument}");
        }
    }
}
