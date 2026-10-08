//! The user's pick: which candidates it names, bringing them in, and what
//! the model is told.

use scorsese_providers::chat::{Call, Part, ResultPart};
use scorsese_providers::stock::{Choice, Medium};
use scorsese_render::Resolution;

use super::read;
use crate::assistant::calls::revision;
use crate::assistant::store::{CandidateView, QuestionView};
use crate::db::UserId;
use crate::events::Event;
use crate::http::AppState;
use crate::tools::Client;

/// How much of the import's answer the browser is shown as it happens.
const SAID: usize = 300;

/// The candidates `keys` name in `question`, in the order they were offered —
/// or why the pick is not one of them, in words for the user.
pub(in crate::assistant) fn chosen(
    question: &QuestionView,
    keys: &[String],
) -> Result<Vec<Choice>, String> {
    if let Some(stranger) = keys
        .iter()
        .find(|key| !question.candidates.iter().any(|one| &one.key == *key))
    {
        return Err(format!("{stranger} is not one of the candidates shown"));
    }
    let picked = question
        .candidates
        .iter()
        .filter(|one| keys.contains(&one.key));
    picked
        .map(|one| choice(one).ok_or_else(|| format!("{} cannot be imported", one.key)))
        .collect()
}

/// The source's result a candidate is.
fn choice(candidate: &CandidateView) -> Option<Choice> {
    let medium = match candidate.kind.as_str() {
        "video" => Medium::Video,
        "image" => Medium::Image,
        _ => return None,
    };
    let id = candidate.id.parse().ok()?;
    Some(Choice { medium, id })
}

/// The result that resumes a turn paused on `call`'s picker: what the user
/// `picked` — imported first, into `project` — or `None` when they answered
/// in `words` alone, and whatever words they wrote.
pub(in crate::assistant) async fn result(
    state: &AppState,
    (user, turn, project): (UserId, i64, i64),
    call: &Call,
    picked: Option<&[Choice]>,
    words: Option<&str>,
) -> Part {
    let mut text = match picked {
        None => "The person picked nothing; they answered in words instead.".to_owned(),
        Some([]) => "The person picked none of these.".to_owned(),
        Some(choices) => {
            let named: Vec<String> = choices
                .iter()
                .map(|one| format!("{} {}", one.medium.word(), one.id))
                .collect();
            let frame = read(&call.input).map_or(Resolution::HD, |asked| asked.frame);
            let brought = fetch(state, (user, turn, project), choices, frame).await;
            format!("The person picked {}. {brought}", named.join(", "))
        }
    };
    if let Some(words) = words {
        text.push_str(&format!("\nThey wrote: {words}"));
    }
    Part::Result {
        call: call.id.clone(),
        name: call.name.clone(),
        content: vec![ResultPart::Text { text }],
        is_error: false,
    }
}

/// Import `choices` as the user's own call, saying so on the event stream as
/// a tool line; what the model is told of it.
async fn fetch(
    state: &AppState,
    (user, turn, project): (UserId, i64, i64),
    choices: &[Choice],
    frame: Resolution,
) -> String {
    let tell = |state_: &'static str, said: Option<String>| {
        let tool = "stock_import".to_owned();
        let event = Event::ChatTool {
            turn,
            tool,
            state: state_,
            said,
        };
        state.events.send(user, event);
    };
    tell("running", None);
    let client = Client::User { turn };
    let frame = (frame.width(), frame.height());
    let outcome = state
        .tools
        .import_stock(user, client, project, choices, frame)
        .await;
    let (ended, words) = match &outcome {
        Ok(lines) => ("answered", lines.clone()),
        Err(why) => ("refused", why.clone()),
    };
    tell(ended, Some(words.chars().take(SAID).collect()));
    if outcome.is_ok()
        && let Some(now) = revision(state, user, project).await
    {
        let event = Event::Project {
            id: project,
            revision: now,
        };
        state.events.send(user, event);
    }
    match outcome {
        Ok(lines) => format!(
            "They are imported into the project already; do not stock_import them again.\n{lines}"
        ),
        Err(why) => format!("Importing them failed, so nothing was imported: {why}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(kind: &str, id: u64) -> CandidateView {
        CandidateView {
            key: format!("pixabay-{kind}-{id}"),
            source: "pixabay".into(),
            kind: kind.into(),
            id: id.to_string(),
            preview_url: String::new(),
            look_url: String::new(),
            width: 1920,
            height: 1080,
            seconds: None,
            author: String::new(),
            page_url: String::new(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn a_pick_names_offered_candidates_in_their_order() {
        let question = QuestionView {
            question: "Which?".into(),
            options: Vec::new(),
            answer: None,
            candidates: vec![
                candidate("video", 1),
                candidate("image", 2),
                candidate("video", 3),
            ],
            picked: None,
        };
        let keys = ["pixabay-video-3".to_owned(), "pixabay-image-2".to_owned()];
        let picked = chosen(&question, &keys).expect("offered");
        let image = Choice {
            medium: Medium::Image,
            id: 2,
        };
        assert_eq!(
            picked,
            [
                image,
                Choice {
                    medium: Medium::Video,
                    id: 3
                }
            ]
        );
        assert_eq!(chosen(&question, &[]), Ok(Vec::new()));
        let stranger = chosen(&question, &["pixabay-video-9".to_owned()]);
        assert!(
            stranger
                .expect_err("not offered")
                .contains("pixabay-video-9")
        );
    }
}
