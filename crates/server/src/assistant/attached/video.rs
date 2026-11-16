//! An attached video, as the model is shown it (#1563): a glance at it, and
//! its transcript when the library has heard it.

use scorsese_providers::chat::ResultPart;

use super::MAX_INLINE_CHARS;
use crate::assistant::pictures;
use crate::db::UserId;
use crate::http::AppState;
use crate::library::Item;

/// A video's line in the note, and a glance at it: its sheet, and its
/// transcript when the library has heard it — or a line saying it could
/// not be shown.
pub(super) async fn video(
    state: &AppState,
    user: UserId,
    item: &Item,
) -> (String, Vec<ResultPart>) {
    let file = state
        .library
        .storage()
        .library_file(user, &item.sha256, &item.extension);
    let words = match state.library.transcript(user, &item.sha256).await {
        Ok(held) => held.words,
        Err(error) => {
            tracing::warn!("assistant: reading video {}'s transcript: {error}", item.id);
            None
        }
    };
    let ffmpeg = state.tools.ffmpeg().clone();
    let glanced =
        tokio::task::spawn_blocking(move || pictures::glanced(&ffmpeg, &file, words.as_ref()))
            .await
            .unwrap_or_else(|error| Err(error.to_string()));
    let (sheet, glance) = match glanced {
        Ok(glanced) => glanced,
        Err(why) => {
            tracing::warn!("assistant: showing video {}: {why}", item.id);
            let line = format!(
                "{} “{}” — a video that could not be prepared to show you",
                item.id, item.name
            );
            return (line, Vec::new());
        }
    };
    let frames = scorsese_render::contact::counted(glance.sheet.at_seconds.len());
    let mut line = format!(
        "{} “{}” — a video: shown in the message as one sheet of {frames} chosen across it, \
         each labelled with its time",
        item.id, item.name
    );
    let mut shown = vec![sheet];
    match glance.transcript {
        Some(lines) => {
            line.push_str(", with its transcript");
            shown.insert(
                0,
                ResultPart::Text {
                    text: transcript(item, &lines),
                },
            );
        }
        None => line.push_str("; it has no transcript"),
    }
    (line, shown)
}

/// A video's transcript as the message carries it, headed with what it is,
/// and cut at a line once it passes [`MAX_INLINE_CHARS`].
fn transcript(item: &Item, lines: &str) -> String {
    let mut kept = String::new();
    let mut cut = false;
    for line in lines.lines() {
        if kept.chars().count() + line.chars().count() > MAX_INLINE_CHARS {
            cut = true;
            break;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    let rest = if cut {
        "\n(cut here: the rest is too long to show in a message)"
    } else {
        ""
    };
    format!(
        "Transcript of attached video {} “{}”, each line led by when it is said, in the same \
         times the sheet's frames are labelled with:\n\n{}{rest}",
        item.id,
        item.name,
        kept.trim_end()
    )
}
