//! What a quote's box shows beside each price: the words that would be sent
//! (#709), so the user never agrees to money without seeing what it buys.
//!
//! A paid tool's reply prices each thing on a line of its own,
//! `subject: price detail` — the shape every paid tool answers in, for every
//! client. The description is looked up here, when the quote is held, from
//! where the tool itself reads it: the project's brief for `generate`, the
//! call's own `prompt` for `voice_design`. Nothing it finds is trusted for
//! money — the token is bound to the briefs as quoted, so a brief edited
//! after this read is refused at the yes and quoted again.

use scorsese_core::AssetKind;
use serde_json::Value;

use super::store::{BriefKind, QuoteItem};
use crate::db::UserId;
use crate::http::AppState;
use crate::projects;

/// `lines` — a quote from `tool`, called with `input` — as the box shows
/// them: an item for each line about something with a description, and every
/// other line (the total, a note) as it was.
pub(super) async fn describe(
    state: &AppState,
    user: UserId,
    tool: &str,
    input: &Value,
    lines: &[String],
) -> (Vec<QuoteItem>, Vec<String>) {
    let briefs = briefs(state, user, tool, input).await;
    let mut items = Vec::new();
    let mut rest = Vec::new();
    for line in lines {
        let found = line.split_once(": ").and_then(|(subject, says)| {
            let (_, brief, description) = briefs.iter().find(|(id, ..)| id == subject)?;
            Some(QuoteItem {
                subject: subject.to_owned(),
                says: says.to_owned(),
                brief: *brief,
                description: description.clone(),
            })
        });
        match found {
            Some(item) => items.push(item),
            None => rest.push(line.clone()),
        }
    }
    (items, rest)
}

/// Every subject `tool` could price, with what it would send. Empty when it
/// cannot be read: the box then shows the quote's lines alone, as before.
async fn briefs(
    state: &AppState,
    user: UserId,
    tool: &str,
    input: &Value,
) -> Vec<(String, BriefKind, String)> {
    let said = |field: &str| input.get(field).and_then(Value::as_str).map(str::to_owned);
    match tool {
        "voice_design" => said("prompt")
            .map(|prompt| vec![("design".to_owned(), BriefKind::Voice, prompt)])
            .unwrap_or_default(),
        "generate" => {
            let Some(project) = input.get("project").and_then(Value::as_i64) else {
                return Vec::new();
            };
            let Ok(stored) = projects::open(&state.pool, user, project).await else {
                return Vec::new();
            };
            stored
                .document
                .assets
                .into_iter()
                // Only the prompted kinds carry a prompt: validation refuses
                // one on every other.
                .filter_map(|asset| {
                    let brief = match asset.kind {
                        AssetKind::GeneratedAudio => BriefKind::Line,
                        _ => BriefKind::Prompt,
                    };
                    Some((asset.id.to_string(), brief, asset.prompt?))
                })
                .collect()
        }
        _ => Vec::new(),
    }
}
