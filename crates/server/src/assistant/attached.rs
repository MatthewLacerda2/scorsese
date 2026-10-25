//! Files attached to a message (#1534): the library ids the chat's tray sends
//! beside the prompt. The media and pages among them are brought into the
//! project before the turn starts — the same `import` the Library modal runs,
//! recorded as the editor's, since the person put them there by hand — and
//! the model is told in a note which files came with this message, so it does
//! not go looking. [`super::start`] brings them in only once its own refusals
//! have passed, so a refused message changes nothing; the import is held to
//! the same per-minute limit as the editor's own edits.
//!
//! What the model **sees** of them (#1562) rides in the message itself, beside
//! the person's words, within a budget, since every later call replays it:
//!
//! - **Pictures**: the first [`MAX_PICTURES`], each a JPEG no longer than
//!   [`pictures::ATTACHED_EDGE`]. The rest, and one that cannot be read as a
//!   picture, are named in the note only.
//! - **Documents**: the text of each, up to [`MAX_INLINE_CHARS`]; a longer one
//!   is named, to read with `library_read` a page at a time.
//! - **Video and sound**: named only. Showing a video to a model is #1563's.
//!
//! The note is the server's, which vouches for it; what a file holds is the
//! person's, so it is never put in the note.

use std::collections::HashSet;

use scorsese_providers::chat::{Part, ResultPart};
use serde_json::json;

use super::{AssistantError, pictures};
use crate::db::UserId;
use crate::events::Event;
use crate::http::AppState;
use crate::library::{Item, Kind, LibraryError};
use crate::projects::{self, ProjectError};
use crate::tools::Refusal;

/// The most files one message carries: more than a tray shows, far fewer
/// than would crowd the model's note.
pub const MAX_ATTACHMENTS: usize = 20;

/// The most pictures one message shows the model.
pub const MAX_PICTURES: usize = 8;

/// The longest document whose text comes in the message, in characters:
/// about 12,000 tokens, a long script whole.
pub const MAX_INLINE_CHARS: usize = 50_000;

/// What a message's attachments bring the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attached {
    /// The server's note naming every file.
    pub note: String,
    /// What it is shown beside the person's words, in the order attached:
    /// each document's text, then each picture.
    pub shown: Vec<ResultPart>,
}

impl Attached {
    /// What is shown, as parts of the person's message.
    pub(super) fn parts(&self) -> impl Iterator<Item = Part> + '_ {
        self.shown.iter().map(|shown| match shown {
            ResultPart::Text { text } => Part::Text { text: text.clone() },
            ResultPart::Image { media_type, data } => Part::Image {
                media_type: media_type.clone(),
                data: data.clone(),
            },
        })
    }
}

/// Bring `items` from `user`'s library into `project`, and what the model is
/// told and shown of them — `None` when nothing is attached. Refused, with
/// nothing imported, when one is not a file of theirs.
pub(super) async fn attach(
    state: &AppState,
    user: UserId,
    project: i64,
    items: &[i64],
) -> Result<Option<Attached>, AssistantError> {
    if items.len() > MAX_ATTACHMENTS {
        return Err(AssistantError::Invalid(format!(
            "a message carries at most {MAX_ATTACHMENTS} files"
        )));
    }
    let mut seen = HashSet::with_capacity(items.len());
    let ids: Vec<i64> = items
        .iter()
        .copied()
        .filter(|id| seen.insert(*id))
        .collect();
    if ids.is_empty() {
        return Ok(None);
    }
    let before = projects::open(&state.pool, user, project)
        .await
        .map_err(opened)?;
    let mut files = Vec::with_capacity(ids.len());
    for id in &ids {
        let item = state
            .library
            .get(user, *id)
            .await
            .map_err(|error| match error {
                LibraryError::NotFound => {
                    AssistantError::Invalid(format!("there is no file {id} in your library"))
                }
                other => AssistantError::Internal(other.to_string()),
            })?;
        files.push(item);
    }
    let media: Vec<i64> = files
        .iter()
        .filter(|item| item.kind != Kind::Text)
        .map(|item| item.id)
        .collect();
    let mut lines = Vec::new();
    if !media.is_empty() {
        state
            .limits
            .take(user, 1)
            .map_err(|wait| AssistantError::TooMany(wait.as_secs().max(1)))?;
        let arguments = json!({ "project": project, "items": media });
        let reply = state
            .tools
            .edit(user, "import", &arguments, None)
            .await
            .map_err(|refusal| match refusal {
                Refusal::Said(why) => AssistantError::Invalid(why),
                moved @ Refusal::Moved => AssistantError::Internal(moved.to_string()),
            })?;
        lines.extend(reply.parts.into_iter().map(|part| part.text));
        let now = projects::open(&state.pool, user, project)
            .await
            .map_err(opened)?;
        if now.summary.revision != before.summary.revision {
            let revision = now.summary.revision;
            state.events.send(
                user,
                Event::Project {
                    id: project,
                    revision,
                },
            );
        }
    }
    let mut texts = Vec::new();
    let mut images = Vec::new();
    for item in &files {
        let (line, shown) = match item.kind {
            Kind::Text => document(state, user, item).await,
            Kind::Image if images.len() < MAX_PICTURES => picture(state, user, item).await,
            Kind::Image => (
                format!(
                    "{} “{}” — not shown to you: a message shows at most {MAX_PICTURES} pictures",
                    item.id, item.name
                ),
                None,
            ),
            Kind::Video | Kind::Audio => (
                format!(
                    "{} “{}” — a {}: not shown to you, only named",
                    item.id,
                    item.name,
                    item.kind.label()
                ),
                None,
            ),
            Kind::Html => continue,
        };
        lines.push(line);
        match shown {
            Some(text @ ResultPart::Text { .. }) => texts.push(text),
            Some(image) => images.push(image),
            None => {}
        }
    }
    texts.append(&mut images);
    Ok(Some(Attached {
        note: note(&lines),
        shown: texts,
    }))
}

/// A document's line in the note, and its text when it is short enough to
/// show whole.
async fn document(state: &AppState, user: UserId, item: &Item) -> (String, Option<ResultPart>) {
    let file = state
        .library
        .storage()
        .library_file(user, &item.sha256, &item.extension);
    match tokio::fs::read_to_string(file).await {
        Ok(text) if text.chars().count() <= MAX_INLINE_CHARS => (
            format!(
                "{} “{}” — a document: its text is in the message",
                item.id, item.name
            ),
            Some(ResultPart::Text {
                text: inline(item, &text),
            }),
        ),
        read => {
            if let Err(error) = read {
                eprintln!(
                    "scorsese-server: assistant: reading document {}: {error}",
                    item.id
                );
            }
            (
                format!(
                    "{} “{}” — a document too long to show whole: read it with library_read",
                    item.id, item.name
                ),
                None,
            )
        }
    }
}

/// A document's text as the message carries it, headed with what it is.
fn inline(item: &Item, text: &str) -> String {
    format!("Attached document {} “{}”:\n\n{text}", item.id, item.name)
}

/// A picture's line in the note, and the picture, shrunk — or a line saying
/// it could not be shown.
async fn picture(state: &AppState, user: UserId, item: &Item) -> (String, Option<ResultPart>) {
    let file = state
        .library
        .storage()
        .library_file(user, &item.sha256, &item.extension);
    let ffmpeg = state.tools.ffmpeg().clone();
    let shrunk = match tokio::fs::read(file).await {
        Ok(bytes) => tokio::task::spawn_blocking(move || pictures::attached(&ffmpeg, &bytes))
            .await
            .unwrap_or_else(|error| Err(error.to_string())),
        Err(error) => Err(error.to_string()),
    };
    match shrunk {
        Ok(image) => (
            format!(
                "{} “{}” — a picture: shown in the message",
                item.id, item.name
            ),
            Some(image),
        ),
        Err(why) => {
            eprintln!(
                "scorsese-server: assistant: showing picture {}: {why}",
                item.id
            );
            (
                format!(
                    "{} “{}” — a picture that could not be prepared to show you",
                    item.id, item.name
                ),
                None,
            )
        }
    }
}

/// The note: `import`'s lines for the media, then one a file saying what of
/// it the model is shown.
fn note(lines: &[String]) -> String {
    format!(
        "The person attached these files from their library to this message; the media are \
         in the project's assets already, so do not import them again:\n{}",
        lines.join("\n")
    )
}

fn opened(error: ProjectError) -> AssistantError {
    match error {
        ProjectError::NotFound => AssistantError::NotFound,
        other => AssistantError::Internal(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_names_each_file_on_a_line_of_its_own() {
        let said = note(&[
            "7 “logo.png” → asset `logo` (added)".into(),
            "9 “a.txt” — a document".into(),
        ]);
        assert!(said.ends_with("\n7 “logo.png” → asset `logo` (added)\n9 “a.txt” — a document"));
    }

    #[test]
    fn what_is_shown_becomes_parts_of_the_persons_message() {
        let attached = Attached {
            note: String::new(),
            shown: vec![
                ResultPart::Text { text: "doc".into() },
                ResultPart::Image {
                    media_type: "image/jpeg".into(),
                    data: "AA==".into(),
                },
            ],
        };
        let parts: Vec<Part> = attached.parts().collect();
        assert_eq!(
            parts,
            [
                Part::Text { text: "doc".into() },
                Part::Image {
                    media_type: "image/jpeg".into(),
                    data: "AA==".into()
                }
            ]
        );
    }
}
