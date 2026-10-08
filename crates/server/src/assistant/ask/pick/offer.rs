//! What a picker shows: only results the turn's own searches listed, each as
//! the modal draws it.

use std::collections::HashSet;

use scorsese_providers::chat::{Message, Part, ResultPart};
use scorsese_providers::stock::{Candidate, Choice, find_cached, named_in};
use serde_json::Value;

use super::{key, read};
use crate::assistant::store::{CandidateView, QuestionView};
use crate::db::UserId;
use crate::http::AppState;

/// The tool whose replies list the results a picker may offer.
const SEARCH: &str = "stock_search";

/// The picker `input` asks for, in `user`'s turn whose messages so far are
/// `record` — or why it cannot be shown, in words for the model.
///
/// Each candidate must be one of this turn's `stock_search` replies listed,
/// and still in the user's stock cache, which is where what the modal shows
/// of it comes from: no request reaches the source here.
pub(in crate::assistant) async fn offer(
    state: &AppState,
    user: UserId,
    record: &[Message],
    input: &Value,
) -> Result<QuestionView, String> {
    let asked = read(input)?;
    let seen = shown(record);
    if let Some(unseen) = asked.candidates.iter().find(|one| !seen.contains(one)) {
        return Err(format!(
            "Not shown: {} {} is not among the results this turn's stock_search calls listed. \
             Offer only results a search showed you, or search again.",
            unseen.medium.word(),
            unseen.id
        ));
    }
    let cache = state.library.storage().stock(user);
    let choices = asked.candidates.clone();
    let found = tokio::task::spawn_blocking(move || {
        let found = choices
            .iter()
            .map(|one| find_cached(&cache, one.medium, one.id));
        found.collect::<Vec<_>>()
    })
    .await
    .map_err(|_| "Not shown: reading the search cache crashed on the server.".to_owned())?;
    let mut candidates = Vec::new();
    for (choice, found) in asked.candidates.iter().zip(found) {
        let Some(found) = found else {
            return Err(format!(
                "Not shown: {} {} is no longer in the search results kept for a day; search \
                 again.",
                choice.medium.word(),
                choice.id
            ));
        };
        candidates.push(view(*choice, &found));
    }
    Ok(QuestionView {
        question: asked.question,
        options: Vec::new(),
        answer: None,
        candidates,
        picked: None,
    })
}

/// Every result a `stock_search` reply in `record` listed — the answered
/// ones; a refusal lists nothing.
fn shown(record: &[Message]) -> HashSet<Choice> {
    let results = record
        .iter()
        .filter_map(|message| match message {
            Message::User { content } => Some(content),
            _ => None,
        })
        .flatten()
        .filter_map(|part| match part {
            Part::Result {
                name,
                content,
                is_error: false,
                ..
            } if name == SEARCH => Some(content),
            _ => None,
        })
        .flatten();
    let words = results.filter_map(|result| match result {
        ResultPart::Text { text } => Some(text),
        ResultPart::Png { .. } => None,
    });
    words.flat_map(|text| named_in(text)).collect()
}

/// `found` as the picker shows it.
fn view(choice: Choice, found: &Candidate) -> CandidateView {
    let largest = found.largest();
    CandidateView {
        key: key(choice),
        source: "pixabay".to_owned(),
        kind: choice.medium.word().to_owned(),
        id: choice.id.to_string(),
        preview_url: found.preview_url.clone(),
        look_url: found
            .renditions
            .first()
            .map_or_else(|| found.preview_url.clone(), |one| one.url.clone()),
        width: largest.map_or(0, |one| one.width),
        height: largest.map_or(0, |one| one.height),
        seconds: found.seconds,
        author: found.author.clone(),
        page_url: found.page_url.clone(),
        tags: found.tags.clone(),
    }
}

#[cfg(test)]
mod tests {
    use scorsese_providers::stock::Medium;

    use super::*;

    fn result(name: &str, text: &str, is_error: bool) -> Message {
        Message::User {
            content: vec![Part::Result {
                call: "toolu_1".into(),
                name: name.into(),
                content: vec![ResultPart::Text { text: text.into() }],
                is_error,
            }],
        }
    }

    #[test]
    fn only_what_an_answered_search_listed_was_shown() {
        let record = [
            Message::User {
                content: vec![Part::Text {
                    text: "1. video 9  12s".into(),
                }],
            },
            result(SEARCH, "1. video 39009  12s  up to 1920x1080  (sun)", false),
            result(SEARCH, "1. video 5  3s  (refused, somehow)", true),
            result("stock_import", "1. image 8", false),
        ];
        let seen = shown(&record);
        let video = |id| Choice {
            medium: Medium::Video,
            id,
        };
        assert_eq!(seen, HashSet::from([video(39009)]));
    }
}
