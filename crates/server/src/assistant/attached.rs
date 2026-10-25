//! Files attached to a message (#1534): the library ids the chat's tray sends
//! beside the prompt. The media and pages among them are brought into the
//! project before the turn starts — the same `import` the Library modal runs,
//! recorded as the editor's, since the person put them there by hand — and
//! the model is told in a note which files came with this message, a
//! document as one to read (`library_read`), so it does not go looking.

use serde_json::json;

use super::AssistantError;
use crate::db::UserId;
use crate::events::Event;
use crate::http::AppState;
use crate::library::{Kind, LibraryError};
use crate::projects::{self, ProjectError};
use crate::tools::Refusal;

/// The most files one message carries: more than a tray shows, far fewer
/// than would crowd the model's note.
pub const MAX_ATTACHMENTS: usize = 20;

/// Bring `items` from `user`'s library into `project`, and the note telling
/// the model of them — `None` when nothing is attached. Refused, with
/// nothing imported, when one is not a file of theirs.
pub async fn attach(
    state: &AppState,
    user: UserId,
    project: i64,
    items: &[i64],
) -> Result<Option<String>, AssistantError> {
    let mut ids: Vec<i64> = Vec::with_capacity(items.len());
    for id in items {
        if !ids.contains(id) {
            ids.push(*id);
        }
    }
    if ids.is_empty() {
        return Ok(None);
    }
    if ids.len() > MAX_ATTACHMENTS {
        return Err(AssistantError::Invalid(format!(
            "a message carries at most {MAX_ATTACHMENTS} files"
        )));
    }
    let before = projects::open(&state.pool, user, project)
        .await
        .map_err(opened)?;
    let mut media = Vec::new();
    let mut documents = Vec::new();
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
        match item.kind {
            Kind::Text => documents.push(format!(
                "{} “{}” — a document: read it with library_read",
                item.id, item.name
            )),
            _ => media.push(item.id),
        }
    }
    let mut lines = Vec::new();
    if !media.is_empty() {
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
    lines.extend(documents);
    Ok(Some(note(&lines)))
}

/// The note: one line a file, each media line `import`'s own.
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
    use super::note;

    #[test]
    fn the_note_names_each_file_on_a_line_of_its_own() {
        let said = note(&[
            "7 “logo.png” → asset `logo` (added)".into(),
            "9 “a.txt” — a document".into(),
        ]);
        assert!(said.ends_with("\n7 “logo.png” → asset `logo` (added)\n9 “a.txt” — a document"));
    }
}
